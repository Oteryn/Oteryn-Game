use super::*;

pub(super) struct Serving<'a, 'f, 's> {
    pub(super) listener: &'a TcpListener,
    pub(super) control: &'a UnixListener,
    pub(super) listener_config: GameplayListenerConfig,
    pub(super) owners: GameplaySeamOwners<'a, 'f, 's>,
    pub(super) material: &'a Material,
    pub(super) readiness: &'a Readiness<'a>,
    pub(super) reporter: Option<super::super::runtime_status::Reporter<SystemClock>>,
    pub(super) shutdown: &'a CancellationToken,
    pub(super) last_position_flush: &'a crate::gameplay_transport::LastPositionFlush,
    pub(super) reset_epoch: u64,
    pub(super) signalled: &'a CancellationToken,
}

pub(super) async fn supervise(serving: Serving<'_, '_, '_>) -> Result<(), BootError> {
    let Serving {
        listener,
        control,
        listener_config,
        owners,
        material,
        readiness,
        reporter,
        shutdown,
        last_position_flush,
        reset_epoch,
        signalled,
    } = serving;
    let root = owners.root;
    let authority = owners.character;
    let proof = owners.holder;
    let evidence = owners.evidence;
    let achievements = owners.achievements;
    let config = &material.config;
    let loops_stop = CancellationToken::new();
    let reset_watch = world_reset::ResetWatch::default();
    let mut gameplay = pin!(world_reset::serve_watching_reset(
        listener,
        listener_config,
        owners,
        shutdown,
        &reset_watch,
        reset_epoch,
    ));
    let mut control_task = pin!(control_loop(
        control,
        root,
        authority,
        proof,
        evidence,
        &material.intents,
        shutdown,
    ));
    let mut expiry = pin!(audit_expiry_loop(root, authority, &loops_stop));
    // Never gates serving; its end is not a shutdown reason. The public
    // projections run beside it, disabled unless configured and enabled.
    let mut projection = pin!(both(
        both(
            both(
                highscores_loop(root, material.world, achievements, &loops_stop),
                super::super::guild_scheduler::run(
                    root,
                    authority,
                    proof,
                    material.world,
                    &loops_stop
                )
            ),
            account_characters_loop(
                root,
                authority,
                material
                    .account_characters
                    .lock()
                    .ok()
                    .and_then(|mut projection| projection.take()),
                &loops_stop
            ),
        ),
        public_projections_loop(
            root,
            authority,
            material
                .public_projections
                .lock()
                .ok()
                .and_then(|mut public| public.take()),
            &loops_stop
        ),
    ));
    let mut projection_done = false;
    // Never gates serving either; disabled unless configured.
    let mut commands = pin!(character_commands_loop(
        root,
        authority,
        proof,
        evidence,
        material
            .character_commands
            .lock()
            .ok()
            .and_then(|mut commands| commands.take()),
        &loops_stop
    ));
    let mut commands_done = false;
    let mut signal = pin!(signalled.cancelled());
    let mut serve_error = None;
    // A loop that ended is never polled again.
    let (mut gameplay_done, mut control_done, mut expiry_done) = (false, false, false);
    let reason = poll_fn(|context| {
        if signal.as_mut().poll(context).is_ready() {
            return Poll::Ready("signal");
        }
        if let Poll::Ready(result) = gameplay.as_mut().poll(context) {
            serve_error = result.err();
            gameplay_done = true;
            return Poll::Ready("gameplay loop ended");
        }
        if control_task.as_mut().poll(context).is_ready() {
            control_done = true;
            return Poll::Ready("control loop ended");
        }
        if expiry.as_mut().poll(context).is_ready() {
            expiry_done = true;
            return Poll::Ready("audit expiry loop ended");
        }
        if !projection_done && projection.as_mut().poll(context).is_ready() {
            projection_done = true;
        }
        if !commands_done && commands.as_mut().poll(context).is_ready() {
            commands_done = true;
        }
        Poll::Pending
    })
    .await;
    event(&format!("event=shutdown reason=\"{reason}\""));
    if let Some(reporter) = &reporter {
        reporter.stop_heartbeats();
    }
    // D3 step 10: withdraw readiness first, within the shutdown budget. The
    // loops keep being driven meanwhile, so work already inside a durability
    // pass finishes and releases the holder the withdrawal needs.
    let budget = tokio::time::Instant::now() + SHUTDOWN_BUDGET;
    // CHAR-POSITION-0 §3.2: the tracked last positions are written while the sessions are still
    // fenced in, so before the withdrawal, within its own share of the budget.
    if !gameplay_done {
        last_position_flush.request();
        let mut flushed = pin!(first(
            last_position_flush.done(),
            tokio::time::sleep(LAST_POSITION_FLUSH_BUDGET),
        ));
        poll_fn(|context| {
            if flushed.as_mut().poll(context).is_ready() {
                return Poll::Ready(());
            }
            if let Poll::Ready(result) = gameplay.as_mut().poll(context) {
                serve_error = result.err();
                gameplay_done = true;
                return Poll::Ready(());
            }
            if !control_done && control_task.as_mut().poll(context).is_ready() {
                control_done = true;
            }
            if !expiry_done && expiry.as_mut().poll(context).is_ready() {
                expiry_done = true;
            }
            if !projection_done && projection.as_mut().poll(context).is_ready() {
                projection_done = true;
            }
            if !commands_done && commands.as_mut().poll(context).is_ready() {
                commands_done = true;
            }
            Poll::Pending
        })
        .await;
    }
    let mut withdrawal = pin!(readiness.publish(false, Some(budget)));
    let withdrawn = poll_fn(|context| {
        if let Poll::Ready(result) = withdrawal.as_mut().poll(context) {
            return Poll::Ready(result);
        }
        if !gameplay_done && let Poll::Ready(result) = gameplay.as_mut().poll(context) {
            serve_error = serve_error.take().or(result.err());
            gameplay_done = true;
        }
        if !control_done && control_task.as_mut().poll(context).is_ready() {
            control_done = true;
        }
        if !expiry_done && expiry.as_mut().poll(context).is_ready() {
            expiry_done = true;
        }
        if !projection_done && projection.as_mut().poll(context).is_ready() {
            projection_done = true;
        }
        if !commands_done && commands.as_mut().poll(context).is_ready() {
            commands_done = true;
        }
        Poll::Pending
    })
    .await;
    if let Some(reporter) = &reporter {
        reporter.withdrawn(withdrawn.as_ref().ok());
    }
    if let Err(error) = withdrawn {
        event(&format!(
            "event=readiness ready=false result=failed reason=\"{error}\""
        ));
    }
    shutdown.cancel();
    loops_stop.cancel();
    // In-flight admissions and an in-flight bootstrap are never cancelled
    // midway: each runs to its own outcome under the bounds of its durability
    // passes and Platform exchanges. The supervisor's stop timeout is the
    // outer bound; a kill after it is recovered like any crash.
    if !gameplay_done {
        let _ = gameplay.as_mut().await;
    }
    if !control_done {
        control_task.as_mut().await;
    }
    if !expiry_done {
        expiry.as_mut().await;
    }
    if !projection_done {
        projection.as_mut().await;
    }
    if !commands_done {
        commands.as_mut().await;
    }
    if let Some(reporter) = reporter {
        reporter
            .finish(budget.saturating_duration_since(tokio::time::Instant::now()))
            .await;
    }
    let _ = std::fs::remove_file(&config.control.socket_path);
    event("event=shutdown state=complete");
    world_reset::classify_exit(&reset_watch, serve_error)
}
