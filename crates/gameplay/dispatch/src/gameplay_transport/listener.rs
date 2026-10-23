use super::*;

pub async fn serve_listener<
    A: FreshAdmissionAuthority,
    I: ConnectionIdentifiers,
    O: ConnectionObserver,
>(
    listener: &TcpListener,
    tls: &Arc<rustls::ServerConfig>,
    limits: ListenerLimits,
    authority: &A,
    identifiers: &I,
    observer: &O,
    shutdown: &CancellationToken,
) {
    serve_listener_with_drain(
        listener,
        tls,
        limits,
        authority,
        identifiers,
        observer,
        shutdown,
        None,
    )
    .await;
}

/// Serve gameplay connections until `shutdown`. Accepting stops at the
/// connection budget (backpressure through the listen backlog). On shutdown the
/// listener stops accepting and entry work is cancelled, while an admission
/// already handed to the owning authority completes before this returns.
#[allow(clippy::too_many_arguments)]
pub(super) async fn serve_listener_with_drain<A, I, O>(
    listener: &TcpListener,
    tls: &Arc<rustls::ServerConfig>,
    limits: ListenerLimits,
    authority: &A,
    identifiers: &I,
    observer: &O,
    shutdown: &CancellationToken,
    drain: Option<&gameplay_drain::GameplayDrain>,
) where
    A: FreshAdmissionAuthority,
    I: ConnectionIdentifiers,
    O: ConnectionObserver,
{
    let units = Semaphore::new(limits.handshake_units);
    let mut connections: Vec<Pin<Box<dyn Future<Output = Option<LostControl>> + '_>>> = Vec::new();
    // Loss/grace lifecycles of ended admitted connections. They hold no socket
    // and at most one exists per admitted session, so they stay outside the
    // connection budget; shutdown cancels them.
    let mut lifecycles: Vec<Pin<Box<dyn Future<Output = ()> + '_>>> = Vec::new();
    let mut stopping = pin!(async {
        match drain {
            Some(drain) => {
                let _ = first(shutdown.cancelled(), drain.requested()).await;
            }
            None => shutdown.cancelled().await,
        }
    });
    let mut stopped = false;
    // After an accept error (e.g. EMFILE) accepting pauses instead of spinning.
    let mut accept_pause: Option<Pin<Box<tokio::time::Sleep>>> = None;
    poll_fn(|context| {
        if !stopped && stopping.as_mut().poll(context).is_ready() {
            stopped = true;
        }
        if let Some(pause) = accept_pause.as_mut()
            && pause.as_mut().poll(context).is_ready()
        {
            accept_pause = None;
        }
        while !stopped && accept_pause.is_none() && connections.len() < limits.connections {
            match listener.poll_accept(context) {
                Poll::Ready(Ok((stream, _))) => connections.push(Box::pin(serve_accepted(
                    stream,
                    tls,
                    &units,
                    limits.entry_deadline,
                    authority,
                    identifiers,
                    observer,
                    shutdown,
                    drain,
                ))),
                Poll::Ready(Err(_)) => {
                    let mut pause = Box::pin(tokio::time::sleep(ACCEPT_ERROR_PAUSE));
                    // Registers the wake-up that resumes accepting.
                    let _ = pause.as_mut().poll(context);
                    accept_pause = Some(pause);
                }
                Poll::Pending => break,
            }
        }
        connections.retain_mut(|connection| match connection.as_mut().poll(context) {
            Poll::Pending => true,
            Poll::Ready(lost) => {
                if let Some(lost) = lost {
                    let mut lifecycle = Box::pin(control_loss_lifecycle(authority, lost, shutdown));
                    // Registers the lifecycle's first wake-up.
                    if lifecycle.as_mut().poll(context).is_pending() {
                        lifecycles.push(lifecycle);
                    }
                }
                false
            }
        });
        lifecycles.retain_mut(|lifecycle| lifecycle.as_mut().poll(context).is_pending());
        if stopped && connections.is_empty() && lifecycles.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn serve_accepted<A, I, O>(
    stream: TcpStream,
    tls: &Arc<rustls::ServerConfig>,
    units: &Semaphore,
    entry_deadline: Duration,
    authority: &A,
    identifiers: &I,
    observer: &O,
    shutdown: &CancellationToken,
    drain: Option<&gameplay_drain::GameplayDrain>,
) -> Option<LostControl>
where
    A: FreshAdmissionAuthority,
    I: ConnectionIdentifiers,
    O: ConnectionObserver,
{
    use connection::ConnectionEnd;
    // Entry work (unit wait, TLS, entry frame) is bounded by the deadline and
    // cancelled by shutdown; nothing authoritative has started yet.
    let entry = async {
        let unit = units.acquire().await.ok()?;
        let mut stream = tcp_tls::accept_tls(stream, tls.clone()).await.ok()?;
        let frame = tcp_tls::read_frame(&mut stream).await.ok()?;
        Some((unit, stream, frame))
    };
    let entry = first(tokio::time::timeout(entry_deadline, entry), async {
        match drain {
            Some(drain) => {
                let _ = first(shutdown.cancelled(), drain.requested()).await;
            }
            None => shutdown.cancelled().await,
        }
    })
    .await;
    let Some(Ok(Some((unit, mut stream, frame)))) = entry else {
        observer.ended(ConnectionEnd::TransportFailed);
        return None;
    };
    // The owning authority's decision is never cancelled midway.
    let admitted = admit_frame(&mut stream, &frame, authority, identifiers).await;
    drop(unit);
    let (end, served) = match admitted {
        // A durably admitted or resumed session whose acceptance could not be delivered
        // still has a controller that is gone: it enters the loss lifecycle, never limbo.
        Err(end @ ConnectionEnd::AdmittedThenDisconnected(_)) => (end, true),
        Err(end) => (end, false),
        Ok(admitted) => match first(
            serve_admitted(&mut stream, admitted, authority, IDLE_LIVENESS),
            async {
                match drain {
                    Some(drain) => {
                        let _ = first(shutdown.cancelled(), drain.requested()).await;
                    }
                    None => shutdown.cancelled().await,
                }
            },
        )
        .await
        {
            Some(end) => (end, true),
            // A shutdown drain is not evidence of lost playable control.
            None => (
                ConnectionEnd::AdmittedThenDisconnected(admitted),
                drain.and_then(|d| d.reset()).is_some(),
            ),
        },
    };
    // Liveness proved the loss: record it now. A closed or failed transport alone is not
    // proof (FND-04B §4): control is lost once the detection window passes unanswered.
    let lost = match (&end, served) {
        (ConnectionEnd::AdmittedThenControlLost(session), true) => Some((*session, Duration::ZERO)),
        (
            ConnectionEnd::AdmittedThenDisconnected(session)
            | ConnectionEnd::AdmittedThenClosed(session, _),
            true,
        ) => Some((
            *session,
            IDLE_LIVENESS
                .interval
                .saturating_mul(IDLE_LIVENESS.missed_limit),
        )),
        _ => None,
    };
    drop(stream);
    observer.ended(end);
    lost.map(|(session, wait)| {
        let reset = drain.and_then(|drain| drain.reset());
        LostControl {
            session,
            wait: if reset.is_some() {
                Duration::ZERO
            } else {
                wait
            },
            reset,
        }
    })
}

/// An ended admitted connection whose control is lost after `wait` without restored control.
pub(super) struct LostControl {
    pub(super) session: AdmittedSession,
    pub(super) wait: Duration,
    pub(super) reset: Option<crate::durability::world_reset::RetiringWorldReset>,
}

/// Durable loss, then grace-expiry release, of one ended admitted connection.
/// Shutdown cancels it; a durable decision already in flight stays atomic.
pub(super) async fn control_loss_lifecycle<A: FreshAdmissionAuthority>(
    authority: &A,
    lost: LostControl,
    shutdown: &CancellationToken,
) {
    let lifecycle = async {
        let mut wait = lost.wait;
        let mut backoff = RECONCILE_BACKOFF;
        loop {
            match authority
                .lose_control(lost.session, wait, lost.reset.as_ref())
                .await
            {
                ControlLossResult::Recorded => {
                    // KILL-REWARD-COMP-1 §1.3: an unknown expiry keeps its fence, kill mark and
                    // parked kills, and runs again behind the backoff, unbounded in count.
                    while authority.expire_control_loss(lost.session).await
                        == GraceExpiryResult::Unknown
                    {
                        tokio::time::sleep(backoff).await;
                        backoff = backoff.saturating_mul(2).min(EXPIRY_MAX_BACKOFF);
                    }
                    break;
                }
                ControlLossResult::ResumedHistory => {
                    let _ = authority.release_abandoned(lost.session).await;
                    break;
                }
                // LOGOUT-WIRE-1 (§1.6): a release committed without its acknowledgement retires
                // the actor; an unreadable row keeps the fence and runs again behind the backoff.
                ControlLossResult::Terminal => {
                    while authority.reconcile_terminal(lost.session).await
                        == GraceExpiryResult::Unknown
                    {
                        tokio::time::sleep(backoff).await;
                        backoff = backoff.saturating_mul(2).min(EXPIRY_MAX_BACKOFF);
                    }
                    break;
                }
                ControlLossResult::Unknown => {
                    // Retained uncertain owner work remains the same operation.
                    // The independently current session ends a superseded loop.
                    tokio::time::sleep(backoff).await;
                    backoff = backoff.saturating_mul(2).min(EXPIRY_MAX_BACKOFF);
                    wait = Duration::ZERO;
                }
                ControlLossResult::NotApplicable | ControlLossResult::Refused => break,
            }
        }
    };
    let _ = first(lifecycle, shutdown.cancelled()).await;
}
