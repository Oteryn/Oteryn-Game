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
        let poll_started = std::time::Instant::now();
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
        let was_full = connections.len() == limits.connections;
        connections.retain_mut(
            |connection| match timed_poll(connection.as_mut(), context) {
                Poll::Pending => true,
                Poll::Ready(lost) => {
                    if let Some(lost) = lost {
                        let mut lifecycle =
                            Box::pin(control_loss_lifecycle(authority, lost, shutdown));
                        // Registers the lifecycle's first wake-up.
                        if lifecycle.as_mut().poll(context).is_pending() {
                            lifecycles.push(lifecycle);
                        }
                    }
                    false
                }
            },
        );
        if was_full && connections.len() < limits.connections && !stopped {
            // Accept was skipped while full. Poll it again after releasing a
            // slot, even when no remaining connection can wake this listener.
            context.waker().wake_by_ref();
        }
        lifecycles.retain_mut(|lifecycle| lifecycle.as_mut().poll(context).is_pending());
        let polled = poll_started.elapsed();
        if polled >= LISTENER_POLL_REPORT {
            operator_event(&format!(
                "event=listener_poll_slow ms={} connections={} lifecycles={}",
                polled.as_millis(),
                connections.len(),
                lifecycles.len(),
            ));
        }
        if stopped && connections.is_empty() && lifecycles.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
}

/// One poll of the shared listener task, or of one connection in it, that runs at least this
/// long is reported (NBDIAG-6): every other connection, including TLS entry, waits meanwhile.
const LISTENER_POLL_REPORT: Duration = Duration::from_millis(500);

/// Polls one connection and reports a poll that kept the shared listener task (NBDIAG-6).
fn timed_poll(
    connection: Pin<&mut (dyn Future<Output = Option<LostControl>> + '_)>,
    context: &mut std::task::Context<'_>,
) -> Poll<Option<LostControl>> {
    let started = std::time::Instant::now();
    let poll = connection.poll(context);
    let polled = started.elapsed();
    if polled >= LISTENER_POLL_REPORT {
        operator_event(&format!(
            "event=listener_connection_poll_slow ms={} ready={}",
            polled.as_millis(),
            poll.is_ready(),
        ));
    }
    poll
}

/// The closed-schema name of how a connection ended before or at admission (NBDIAG-6).
fn connection_end_kind(end: &connection::ConnectionEnd) -> &'static str {
    use connection::{AdmissionRefusal, ConnectionEnd};
    match end {
        ConnectionEnd::ProtocolViolation(_) => "protocol_violation",
        ConnectionEnd::AdmissionRefused(AdmissionRefusal::PremiumRequired) => "refused_premium",
        ConnectionEnd::AdmissionRefused(AdmissionRefusal::Rejected) => "refused_rejected",
        ConnectionEnd::AdmissionRefused(AdmissionRefusal::Unavailable) => "refused_unavailable",
        ConnectionEnd::AdmissionRefused(AdmissionRefusal::Classified(_)) => "refused_classified",
        ConnectionEnd::AdmissionRefused(AdmissionRefusal::Queued(_)) => "refused_queued",
        ConnectionEnd::AdmissionRefused(AdmissionRefusal::RecoveryCapabilityRequired) => {
            "refused_recovery_capability"
        }
        ConnectionEnd::ResumeUnavailable => "resume_unavailable",
        ConnectionEnd::AdmittedThenClosed(..) => "admitted_then_closed",
        ConnectionEnd::AdmittedThenDisconnected(_) => "admitted_then_disconnected",
        ConnectionEnd::AdmittedThenControlLost(_) => "admitted_then_control_lost",
        ConnectionEnd::AdmittedThenLoggedOut(_) => "admitted_then_logged_out",
        ConnectionEnd::TransportFailed => "transport_failed",
    }
}

fn report_transport_close(
    end: &connection::ConnectionEnd,
    served: bool,
    elapsed: Duration,
    trace: Option<[u8; 16]>,
) {
    let mut line = oteryn_error_codes::Line::new(
        oteryn_error_codes::Level::Info,
        "gameplay_transport",
        "transport_close",
    )
    .field(
        "outcome",
        oteryn_error_codes::FieldValue::Token(connection_end_kind(end)),
    )
    .field(
        "served",
        oteryn_error_codes::FieldValue::Token(if served { "true" } else { "false" }),
    )
    .field(
        "ms",
        oteryn_error_codes::FieldValue::U64(u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)),
    );
    let reason = match end {
        connection::ConnectionEnd::ProtocolViolation(error)
        | connection::ConnectionEnd::AdmittedThenClosed(_, error) => Some(*error),
        connection::ConnectionEnd::AdmissionRefused(connection::AdmissionRefusal::Classified(
            error,
        )) => Some(*error),
        _ => None,
    };
    if let Some(error) = reason {
        line = line.field(
            "reason_code",
            oteryn_error_codes::FieldValue::U64(u64::from(error.code())),
        );
    }
    let trace = trace.map(format_connection_trace);
    if let Some(trace) = trace.as_deref() {
        line = line.trace(trace);
    }
    if let connection::ConnectionEnd::AdmittedThenClosed(session, _)
    | connection::ConnectionEnd::AdmittedThenDisconnected(session)
    | connection::ConnectionEnd::AdmittedThenControlLost(session)
    | connection::ConnectionEnd::AdmittedThenLoggedOut(session) = end
    {
        line = line.session_gen(session.continuity.connection_generation);
    }
    oteryn_error_codes::emit("oteryn-game-server", &line);
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
    let trace = crate::operator_files::uuid_v7().ok();
    // Entry work (unit wait, TLS, entry frame) is bounded by the deadline and
    // cancelled by shutdown; nothing authoritative has started yet.
    // NBDIAG-6: which entry step a failed or slow entry was waiting for.
    let entry_started = tokio::time::Instant::now();
    let entry_step = std::sync::atomic::AtomicU8::new(0);
    let entry = async {
        let unit = units.acquire().await.ok()?;
        entry_step.store(1, std::sync::atomic::Ordering::Relaxed);
        let mut stream = tcp_tls::accept_tls(stream, tls.clone()).await.ok()?;
        entry_step.store(2, std::sync::atomic::Ordering::Relaxed);
        let frame = tcp_tls::read_frame(&mut stream).await.ok()?;
        entry_step.store(3, std::sync::atomic::Ordering::Relaxed);
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
    let entry_step = match entry_step.load(std::sync::atomic::Ordering::Relaxed) {
        0 => "unit",
        1 => "tls",
        2 => "frame",
        _ => "done",
    };
    let entry_ms = entry_started.elapsed().as_millis();
    let outcome = match &entry {
        None => "shutdown",
        Some(Err(_)) => "deadline",
        Some(Ok(None)) => "failed",
        Some(Ok(Some(_))) => "entered",
    };
    let Some(Ok(Some((unit, mut stream, frame)))) = entry else {
        operator_event_with_trace(
            &format!(
                "event=connection_entry_failed step={entry_step} outcome={outcome} ms={entry_ms}"
            ),
            trace,
        );
        observer.ended(ConnectionEnd::TransportFailed);
        return None;
    };
    if entry_ms >= LISTENER_POLL_REPORT.as_millis() {
        operator_event_with_trace(&format!("event=connection_entry_slow ms={entry_ms}"), trace);
    }
    // The owning authority's decision is never cancelled midway.
    if cast_observer::ingress(&frame) {
        if let Some(node) = authority.cast_node() {
            let _ = cast_observer::serve(stream, &frame, node, shutdown, unit).await;
        }
        observer.ended(ConnectionEnd::AdmissionRefused(
            connection::AdmissionRefusal::Unavailable,
        ));
        return None;
    }
    let admit_started = tokio::time::Instant::now();
    let admitted = match trace {
        Some(trace) => {
            connection::with_connection_trace(
                trace,
                admit_frame(&mut stream, &frame, authority, identifiers),
            )
            .await
        }
        None => admit_frame(&mut stream, &frame, authority, identifiers).await,
    };
    drop(unit);
    match &admitted {
        Err(end) => operator_event_with_trace(
            &format!(
                "event=connection_admit_failed end={} ms={}",
                connection_end_kind(end),
                admit_started.elapsed().as_millis(),
            ),
            trace,
        ),
        Ok(_) => operator_event_with_trace(
            &format!(
                "event=connection_admitted ms={}",
                admit_started.elapsed().as_millis()
            ),
            trace,
        ),
    }
    let (end, served) = match admitted {
        // A durably admitted or resumed session whose acceptance could not be delivered
        // still has a controller that is gone: it enters the loss lifecycle, never limbo.
        Err(end @ ConnectionEnd::AdmittedThenDisconnected(_)) => (end, true),
        Err(end) => (end, false),
        Ok(admitted) => match first(
            async {
                match trace {
                    Some(trace)
                        if admitted.continuity.selected_capabilities.contains(
                            oteryn_protocol_oteryn::connection_trace::CAPABILITY_CONNECTION_TRACE_V1,
                        ) => connection::with_selected_connection_trace(
                            trace,
                            serve_admitted(&mut stream, admitted, authority, IDLE_LIVENESS),
                        )
                        .await,
                    Some(_) | None => {
                        serve_admitted(&mut stream, admitted, authority, IDLE_LIVENESS).await
                    }
                }
            },
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
                ConnectionEnd::AdmittedThenDisconnected(
                    authority.current_activity_connection(admitted),
                ),
                drain.and_then(|d| d.reset()).is_some(),
            ),
        },
    };
    report_transport_close(&end, served, admit_started.elapsed(), trace);
    if let ConnectionEnd::AdmittedThenClosed(session, _)
    | ConnectionEnd::AdmittedThenDisconnected(session)
    | ConnectionEnd::AdmittedThenControlLost(session)
    | ConnectionEnd::AdmittedThenLoggedOut(session) = &end
    {
        authority.cast_end(*session).await;
    }
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
    // This physical socket has no subsequent loss/grace owner. Keep its
    // root through the whole serve future (including shutdown), then drop
    // only local aliases after the listener has selected the final result.
    if lost.is_none()
        && let ConnectionEnd::AdmittedThenClosed(session, _)
        | ConnectionEnd::AdmittedThenDisconnected(session)
        | ConnectionEnd::AdmittedThenControlLost(session)
        | ConnectionEnd::AdmittedThenLoggedOut(session) = end
    {
        authority.finish_activity_connection_lineage(session);
    }
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
    // Cancellation may fall between durable handoff and barrier publication.
    // Resolve retained owner work before losing control of an exact successor.
    let settle = async {
        let mut backoff = RECONCILE_BACKOFF;
        loop {
            match authority.settle_activity_connection(lost.session).await {
                Ok(session) => break session,
                Err(()) => {
                    tokio::time::sleep(backoff).await;
                    backoff = backoff.saturating_mul(2).min(EXPIRY_MAX_BACKOFF);
                }
            }
        }
    };
    let Some(session) = first(settle, shutdown.cancelled()).await else {
        return;
    };
    let lost = LostControl { session, ..lost };
    let scoped = match authority.resolve_native_scope(lost.session).await {
        Ok(value) => value,
        Err(()) => return,
    };
    let authority = scoped.as_ref().unwrap_or(authority);
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
                    authority.finish_activity_connection_lineage(lost.session);
                    break;
                }
                ControlLossResult::ResumedHistory => {
                    if authority.release_abandoned(lost.session).await != GraceExpiryResult::Unknown
                    {
                        authority.finish_activity_connection_lineage(lost.session);
                    }
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
                    authority.finish_activity_connection_lineage(lost.session);
                    break;
                }
                ControlLossResult::Unknown => {
                    // Retained uncertain owner work remains the same operation.
                    // The independently current session ends a superseded loop.
                    tokio::time::sleep(backoff).await;
                    backoff = backoff.saturating_mul(2).min(EXPIRY_MAX_BACKOFF);
                    wait = Duration::ZERO;
                }
                ControlLossResult::NotApplicable => {
                    authority.finish_activity_connection_lineage(lost.session);
                    break;
                }
                ControlLossResult::Refused => {
                    // This definite refusal also ends the listener's callback;
                    // only Unknown retains a retry. Current control and native
                    // attempts stay owned independently of these local aliases.
                    authority.finish_activity_connection_lineage(lost.session);
                    break;
                }
            }
        }
    };
    let _ = first(lifecycle, shutdown.cancelled()).await;
}

#[cfg(test)]
mod logging_tests {
    use super::*;

    #[test]
    fn admission_refusal_and_transport_close_emit_registered_events() {
        let lines = oteryn_error_codes::capture(|| {
            connection::report_admission_refusal(connection::AdmissionRefusal::Unavailable);
            report_transport_close(
                &connection::ConnectionEnd::TransportFailed,
                false,
                Duration::from_millis(7),
                None,
            );
        });
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("level=warn module=admission event=admission_refused"));
        assert!(lines[0].contains("kind=unavailable"));
        assert!(lines[1].contains(
            "level=info module=gameplay_transport event=transport_close \
                 outcome=transport_failed served=false ms=7"
        ));
    }

    #[test]
    fn classified_close_diagnostics_retain_reason_codes() {
        let error = FoundationProtocolError::MalformedEnvelope;
        let lines = oteryn_error_codes::capture(|| {
            connection::report_admission_refusal(connection::AdmissionRefusal::Classified(error));
            report_transport_close(
                &connection::ConnectionEnd::ProtocolViolation(error),
                false,
                Duration::ZERO,
                None,
            );
        });
        let reason = format!("reason_code={}", error.code());
        assert!(lines.iter().all(|line| line.contains(&reason)));
    }

    #[test]
    fn monitored_operator_diagnostic_is_a_structured_event() {
        let trace = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
        let lines = oteryn_error_codes::capture(|| {
            operator_event_with_trace(
                "event=serve_end reason=control_lost server_sequence=23",
                Some(trace),
            );
        });
        assert!(lines[0].contains("event=serve_end"));
        assert!(lines[0].contains("reason=control_lost server_sequence=23"));
        assert!(lines[0].contains("trace=01020304-0506-7008-800a-0b0c0d0e0f10"));
        assert!(!lines[0].contains("event=transport_diagnostic"));
    }
}
