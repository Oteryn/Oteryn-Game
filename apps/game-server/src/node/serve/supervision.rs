use super::*;
use crate::map::boot::BundleCollisionIndexExt;

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
    pub(super) generation: u64,
    pub(super) signalled: &'a CancellationToken,
    pub(super) timed_npcs: bundle_npcs::TimedNpcSpawns,
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
        generation,
        signalled,
        timed_npcs,
    } = serving;
    let root = owners.root;
    let authority = owners.character;
    let proof = owners.holder;
    let evidence = owners.evidence;
    let achievements = owners.achievements;
    let native = owners
        .active_generation
        .and_then(|active| active.native_gameplay());
    let runtime = owners.runtime;
    let config = &material.config;
    let (active_layers_epoch, active_layers) =
        super::super::raid_scheduler::world_change_layers(owners.movement_cells);
    let raids = super::super::raid_scheduler::RaidScheduler {
        root,
        character: authority,
        node: proof,
        runtime,
        map: match owners.movement_cells.index() {
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) => {
                Some(index.map())
            }
            _ => None,
        },
        world: material.world,
        channel: material.channel,
        generation,
        reset_epoch,
        active_layers_epoch,
        active_layers,
        bundle_digest: config
            .world_bundle
            .as_ref()
            .and_then(|bundle| bundle.digest_bytes()),
    };
    let loops_stop = CancellationToken::new();
    let reset_watch = world_reset::ResetWatch::default();
    // Box each long-lived family before the supervisor retains it. Inline loop
    // futures also inflate the enclosing boot/run construction and poll frames,
    // leaving too little of the node stack for a player's first command.
    let mut gameplay = Box::pin(world_reset::serve_watching_reset(
        listener,
        listener_config,
        owners,
        shutdown,
        &reset_watch,
        reset_epoch,
    ));
    let mut control_task = Box::pin(control_loop(
        control,
        root,
        authority,
        proof,
        evidence,
        &material.intents,
        config,
        native,
        shutdown,
    ));
    let mut expiry = Box::pin(both(
        both(
            audit_expiry_loop(root, authority, &loops_stop),
            online_statistics_retention_loop(root, authority, &loops_stop),
        ),
        both(
            coin_worker::run(root, authority, proof, material.world, &loops_stop),
            journal_worker::run(root, authority, config, &loops_stop),
        ),
    ));
    // Never gates serving; its end is not a shutdown reason. The public
    // projections run beside it, disabled unless configured and enabled.
    let collector = audit_collector::Collector {
        root,
        authority,
        proof,
        config: &config.audit_collector,
        t94_key_id: material
            .world_event_outcomes
            .lock()
            .ok()
            .and_then(|value| value.as_ref().and_then(|value| value.audit_key_id.clone())),
    };
    let mut projection = Box::pin(both(
        both(
            both(
                both(
                    highscores_loop(root, material.world, achievements, &loops_stop),
                    both(
                        both(
                            super::super::guild_scheduler::run(
                                root,
                                authority,
                                proof,
                                material.world,
                                &loops_stop,
                            ),
                            super::super::house_rent_scheduler::run(
                                root,
                                authority,
                                proof,
                                RuntimeScopeRefV1::channel(material.world, material.channel),
                                generation,
                                &loops_stop,
                            ),
                        ),
                        super::super::market_scheduler::run(
                            root,
                            authority,
                            proof,
                            material.world,
                            &loops_stop,
                        ),
                    ),
                ),
                super::super::raid_scheduler::run(raids, &loops_stop),
            ),
            account_characters_loop(
                root,
                authority,
                material
                    .account_characters
                    .lock()
                    .ok()
                    .and_then(|mut projection| projection.take()),
                &loops_stop,
            ),
        ),
        both(
            both(
                collector.run(&loops_stop),
                t94::run(
                    root,
                    authority,
                    material
                        .world_event_outcomes
                        .lock()
                        .ok()
                        .and_then(|mut value| value.take()),
                    material.world,
                    &loops_stop,
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
                &loops_stop,
            ),
        ),
    ));
    let mut projection_done = false;
    // Never gates serving either; disabled unless configured.
    let mut commands = Box::pin(both(
        character_commands_loop(
            root,
            authority,
            proof,
            evidence,
            material
                .character_commands
                .lock()
                .ok()
                .and_then(|mut commands| commands.take()),
            &loops_stop,
        ),
        both(
            bazaar_loop(
                root,
                authority,
                proof,
                evidence,
                material
                    .bazaar
                    .lock()
                    .ok()
                    .and_then(|mut bazaar| bazaar.take()),
                &loops_stop,
            ),
            both(
                timed_npc_loop(root, material.world, runtime, timed_npcs, &loops_stop),
                account_closure_offline_loop(
                    root,
                    authority,
                    proof,
                    runtime,
                    (material.world, material.channel),
                    &loops_stop,
                ),
            ),
        ),
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

async fn timed_npc_loop(
    root: &DurabilityRoot,
    world: WorldId,
    runtime: &Mutex<ChannelRuntimeV1>,
    mut timed_npcs: bundle_npcs::TimedNpcSpawns,
    stop: &CancellationToken,
) {
    if timed_npcs.is_empty() {
        return;
    }
    let mut tick =
        tokio::time::interval(Duration::from_millis(crate::world_clock::TIBIAN_MINUTE_MS));
    loop {
        if first(tick.tick(), stop.cancelled()).await.is_none() {
            return;
        }
        let Some(now) = crate::world_clock::now_utc_ms() else {
            continue;
        };
        // Crystal data-global/scripts/lib/a_piece_of_cake_config.lua:456-483: a committed next-save
        // flag controls appearance/removal; transient read failure keeps known state.
        match root.cake_brutus_present(world).await {
            Ok(present) => timed_npcs.set_brutus_present(present),
            Err(_) => event("event=cake_brutus_projection state=retry"),
        }
        if timed_npcs
            .refresh_event_contacts(root, world)
            .await
            .is_err()
        {
            event("event=npc_event_presence state=retry");
        }
        let mut runtime = runtime.lock().await;
        if let Some(Err(error)) = timed_npcs.tick(&mut runtime, now) {
            event(&format!(
                "event=timed_npc_tick state=retry reason={error:?}"
            ));
        }
    }
}

#[path = "coin_worker.rs"]
mod coin_worker;

#[path = "journal_worker.rs"]
mod journal_worker;

#[cfg(test)]
mod tests {
    use super::{Serving, supervise};

    #[test]
    fn supervisor_does_not_embed_long_lived_loop_futures() {
        // Boot and run retain this future in their own state. Inspect its real
        // layout without constructing it on the test stack or starting any loop.
        // This bound supplements the running-node qualification, which exercises
        // the nested poll stack and actual first-control commands.
        fn future_size<F: std::future::Future>(
            _: impl FnOnce(Serving<'static, 'static, 'static>) -> F,
        ) -> usize {
            std::mem::size_of::<F>()
        }

        let bytes = future_size(supervise);
        assert!(
            bytes <= 1024 * 1024,
            "supervisor future retained {bytes} bytes of inline state"
        );
    }
}
