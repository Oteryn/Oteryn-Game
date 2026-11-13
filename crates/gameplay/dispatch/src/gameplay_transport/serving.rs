use super::*;

/// Constructed only here after the listener and physical owner jobs have joined
/// (or been dropped) and the shared input gate has drained. A disabled ready
/// guard or a source-cycle cancellation alone cannot construct this proof.
pub(super) struct QuiescedKillOwners(());

/// The production gameplay seam: TCP + TLS 1.3 (ALPN `oteryn-game/1`),
/// bounded FND-02 framing and fresh admission through the composed owners,
/// served on `listener` until `shutdown`.
pub async fn serve_gameplay(
    listener: &TcpListener,
    config: GameplayListenerConfig,
    owners: GameplaySeamOwners<'_, '_, '_>,
    shutdown: &CancellationToken,
    drain: Option<&gameplay_drain::GameplayDrain>,
) -> Result<(), GameplayServeError> {
    let ordinary = gameplay_drain::GameplayDrain::default();
    if config.server_motd.len() > oteryn_protocol_oteryn::zones_nologout::MAX_SERVER_MOTD_BYTES {
        return Err(GameplayServeError::InvalidLimits);
    }
    let drain = drain.unwrap_or(&ordinary);
    let limits = ListenerLimits::new(
        config.max_connections,
        config.max_handshake_units,
        config.entry_deadline,
    )
    .ok_or(GameplayServeError::InvalidLimits)?;
    let tls = tcp_tls::tls_config(config.certificates, config.private_key)
        .map_err(GameplayServeError::InvalidTls)?;
    let casts = std::sync::OnceLock::new();
    if let Some(config) = owners.cast {
        let service = oteryn_gameplay_social::cast::service::CastService::spawn(
            owners.root.clone(),
            owners.holder.clone(),
            config.node_capacity,
            config.access.clone(),
        )
        .map_err(|_| GameplayServeError::InvalidLimits)?;
        let _ = casts.set(cast::CastNode {
            service,
            access: config.access,
        });
    }
    // Keep the sealed independent activity maps alive for the complete gameplay
    // seam. They use the current native Item identities without changing Channel
    // geometry or obtaining an Instance allocation.
    let activity_source_rooms: Vec<_> = owners
        .active_generation
        .zip(owners.rooms.gate_native_combat_cast())
        .into_iter()
        .flat_map(|(active, room)| {
            use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
            PinnedActivitySourceRoom::ALL
                .into_iter()
                .filter_map(move |source| {
                    active
                        .qualify_activity_source_room(room, source)
                        .ok()
                        .and_then(|room| {
                            activity_runtime::source_bundle::qualify(
                                &room,
                                source,
                                active,
                                owners.world_id,
                                owners.channel_id,
                            )
                            .ok()
                        })
                        .map(|room| (source, room))
                })
        })
        .collect();
    let authority = ComposedFreshAdmission {
        server_motd: &config.server_motd,
        reset_epoch: drain.epoch(),
        emit: owners.emit,
        root: owners.root,
        character: owners.character,
        holder: owners.holder,
        evidence: owners.evidence,
        world_id: owners.world_id,
        channel_id: owners.channel_id,
        premium_only: owners.premium_only,
        required_capabilities: owners.required_capabilities,
        runtime: owners.runtime.into(),
        movement_cells: owners.movement_cells.into(),
        door: owners.door.into(),
        chest: owners.chest,
        spells: owners.spells,
        active_generation: owners.active_generation,
        premium_coordinator: owners.premium_coordinator,
        rooms: owners.rooms,
        achievements: owners.achievements,
        imported_charms: owners.imported_charms,
        proficiency_catalogue: weapon_proficiency::ProductionProficiencyCatalogue::embedded(),
        clock_origin: std::time::Instant::now(),
        #[cfg(test)]
        test_step_wait_receipts: None,
        activity_runtimes: Default::default(),
        states: std::sync::Arc::new(channel_states::ChannelSessionStates {
            casts,
            bulletin_source: config.bulletin_source,
            gameplay_idle_policy: owners.gameplay_idle_policy,
            ..Default::default()
        }),
        revision_sequencer:
            crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new()
                .into(),
        quest_catalogue: Some(std::sync::Arc::clone(owners.quest_catalogue)),
        quest_log_content: Some(owners.quest_log_content.clone()),
        // WHEEL-W1: the embedded Wheel ruleset; without it every Wheel load fails closed.
        wheel_ruleset: crate::wheel_gem_data::WheelGemData::embedded()
            .ok()
            .and_then(|data| data.wheel_ruleset().ok())
            .map(std::sync::Arc::new),
        // PREM-WIRE-1: one Premium fence. The refresher pulls into the consumer the spell casts
        // and every other Premium gate read; a node without the Premium reads keeps its own.
        premium: premium_refresher(
            owners.root,
            owners
                .premium_coordinator
                .map_or_else(Default::default, |reads| reads.consumer()),
        ),
        spell_lane: crate::durability::spell_owner_commit::SpellLane::new(
            owners.world_id,
            owners.channel_id,
        ),
        store: owners
            .store
            .map(|client| std::sync::Arc::new(tokio::sync::Mutex::new(client))),
        store_purchases: owners.store_purchases,
    };
    // Capture actual registered source execution time before startup awaits,
    // rather than anchoring a GlobalEvent to its first Think or a boss birth.
    let registered_global_us = authority.owner_now().get();
    authority
        .register_world_spite_global(registered_global_us)
        .await
        .map_err(|_| GameplayServeError::OwnerUnavailable)?;
    authority
        .states
        .boss_activity_handoff
        .install(std::sync::Arc::new(soulpit::SharedSoulpitOwner))
        .map_err(|_| GameplayServeError::OwnerUnavailable)?;
    authority
        .register_current_runtime()
        .await
        .map_err(|_| GameplayServeError::OwnerUnavailable)?;
    for template in owners.activity_templates {
        authority
            .activity_runtimes
            .register_template(template)
            .map_err(|_| GameplayServeError::OwnerUnavailable)?;
    }
    if let Some(template) = owners.soulpit_template {
        authority
            .activity_runtimes
            .register_soulpit_template(template)
            .map_err(|_| GameplayServeError::OwnerUnavailable)?;
    }
    for (source, room) in &activity_source_rooms {
        authority
            .activity_runtimes
            .register_source_room(*source, room)
            .map_err(|_| GameplayServeError::OwnerUnavailable)?;
    }
    if let Some(book) = owners.npc_talk {
        authority.states.chat.install_npc_talk(book);
    }
    // CHAT-2B: the World chat relay, when its key file is configured.
    if let Some(relay) = chat_relay::configured(
        *owners.world_id.as_bytes(),
        owners.root.chat_relay_connect_options(),
    ) {
        authority
            .states
            .chat
            .install_relay(std::sync::Arc::new(relay));
    }
    // N009/N014: the Platform stream pullers run until this seam ends; dropping them aborts them.
    let _platform_streams = platform_stream_pullers(owners.root);
    // Check every initial boss clock before accepting connections; confirmed claims may wait for a free tile.
    let mut boss_timers = std::collections::BTreeMap::new();
    loop {
        if authority
            .boss_spawn_pass(&mut boss_timers)
            .await
            .map_err(|_| GameplayServeError::OwnerUnavailable)?
        {
            break;
        }
        if world_respawns::stopping_first(tokio::time::sleep(Duration::from_millis(25)), shutdown)
            .await
            .is_none()
        {
            return Ok(());
        }
    }
    // QUEST-STATE-0 §5.4: the owner cadence requests failed quest obligations again, whether or
    // not the session's connection runs any other cadence. It never ends on its own.
    let quest_retries = async {
        loop {
            if first(
                tokio::time::sleep(QUEST_RETRY_CADENCE),
                shutdown.cancelled(),
            )
            .await
            .is_none()
            {
                std::future::pending::<()>().await;
            }
            authority.refresh_due_quest_sessions().await;
        }
    };
    let serving = first(
        listener::serve_listener_with_drain(
            listener,
            &tls,
            limits,
            &authority,
            &SecureIdentifiers,
            &(),
            shutdown,
            Some(drain),
        ),
        DurabilityRoot::with_background_holder(quest_retries),
    );
    let mut serving = pin!(serving);
    let cycles_stop = CancellationToken::new();
    let mut cycles = pin!(DurabilityRoot::with_background_holder(
        authority.run_source_owner_cycles(&cycles_stop)
    ));
    let mut activities = pin!(DurabilityRoot::with_background_holder(
        authority.run_activity_owner_cycles(&cycles_stop)
    ));
    let mut activities_done = false;
    let mut respawns = Box::pin(DurabilityRoot::with_background_holder(
        authority.run_world_respawns(shutdown),
    ));
    let mut respawns_done = false;
    let mut owner_failed = false;
    let mut countdown = pin!(authority.run_server_save_countdown(shutdown));
    let mut countdown_done = false;
    let mut quiesce = pin!(async {
        drain.requested().await;
        authority.states.input_gate.quiesce().await;
    });
    let mut quiesced = false;
    // CHAR-POSITION-0 §3.2 (`CHARPOS0-RL-01`): the periodic last-position writes run on their own
    // deadlines, so no other cadence work delays them.
    let mut positions = pin!(DurabilityRoot::with_background_holder(
        authority.run_last_position_writes(shutdown)
    ));
    // CHAR-POSITION-0 §3.2: the shutdown write is polled with the seam and never holds its end.
    let mut flush = pin!(async {
        match owners.last_position_flush {
            Some(flush) => flush.serve(&authority).await,
            None => std::future::pending().await,
        }
    });
    // CHAT-2B: the relay listener; it parks once shutdown is requested.
    let mut relay = pin!(async {
        match authority.states.chat.relay() {
            Some(relay) => relay.run(&authority.states.chat, shutdown).await,
            None => std::future::pending().await,
        }
    });
    let mut serving_done = false;
    let mut cycles_done = false;
    let mut flush_done = false;
    poll_fn(|context| {
        if !respawns_done && let Poll::Ready(exit) = respawns.as_mut().poll(context) {
            respawns_done = true;
            if exit != crate::monster_owner_cycle::respawn_job::RespawnExit::Shutdown {
                owner_failed = true;
                return Poll::Ready(());
            }
        }
        if !countdown_done && countdown.as_mut().poll(context).is_ready() {
            countdown_done = true;
        }
        if !quiesced && quiesce.as_mut().poll(context).is_ready() {
            quiesced = true;
        }
        if shutdown.is_cancelled() {
            cycles_stop.cancel();
        }
        if !flush_done && flush.as_mut().poll(context).is_ready() {
            flush_done = true;
        }
        // Never ends on its own; it parks once shutdown is requested.
        let _ = positions.as_mut().poll(context);
        // Never ends: it parks once shutdown is requested.
        let _ = relay.as_mut().poll(context);
        if !serving_done && serving.as_mut().poll(context).is_ready() {
            serving_done = true;
            cycles_stop.cancel();
        }
        if !cycles_done && cycles.as_mut().poll(context).is_ready() {
            cycles_done = true;
        }
        if !activities_done && activities.as_mut().poll(context).is_ready() {
            activities_done = true;
        }
        if serving_done && cycles_done && activities_done {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
    // The listener does not finish until all connections and control-loss
    // lifecycles finish. Both source/activity producer job groups also joined
    // above. Drop the remaining boxed respawn/raid/Soulpit future so none of
    // its local child owners can be polled while the terminal SQL awaits.
    drop(respawns);
    if !owner_failed && serving_done && cycles_done && activities_done {
        authority.states.input_gate.quiesce().await;
        let quiesced = QuiescedKillOwners(());
        authority.finish_daily_kill_statistics(&quiesced).await;
        // Activity views share this same closed input gate. Their autonomous
        // jobs have joined too; include each still-registered Instance queue.
        if let Ok(owners) = authority.activity_runtimes.activity_owners() {
            for owner in owners {
                authority
                    .runtime_view(&owner)
                    .finish_daily_kill_statistics(&quiesced)
                    .await;
            }
        }
    }
    if let Some(node) = authority.states.casts.get() {
        let _ = node
            .service
            .apply(oteryn_gameplay_social::cast::service::Operation::Shutdown)
            .await;
    }
    if owner_failed {
        Err(GameplayServeError::OwnerUnavailable)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "serving/kill_statistics_shutdown_tests.rs"]
mod kill_statistics_shutdown_tests;
