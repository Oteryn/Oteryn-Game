use super::*;

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
    let drain = drain.unwrap_or(&ordinary);
    let limits = ListenerLimits::new(
        config.max_connections,
        config.max_handshake_units,
        config.entry_deadline,
    )
    .ok_or(GameplayServeError::InvalidLimits)?;
    let tls = tcp_tls::tls_config(config.certificates, config.private_key)
        .map_err(GameplayServeError::InvalidTls)?;
    let authority = ComposedFreshAdmission {
        emit: owners.emit,
        root: owners.root,
        character: owners.character,
        holder: owners.holder,
        evidence: owners.evidence,
        world_id: owners.world_id,
        channel_id: owners.channel_id,
        required_capabilities: owners.required_capabilities,
        runtime: owners.runtime,
        movement_cells: owners.movement_cells,
        door: owners.door,
        chest: owners.chest,
        spells: owners.spells,
        active_generation: owners.active_generation,
        premium_coordinator: owners.premium_coordinator,
        rooms: owners.rooms,
        achievements: owners.achievements,
        imported_charms: owners.imported_charms,
        spell_states: Mutex::default(),
        world_gate: std::sync::Mutex::default(),
        attack: Mutex::default(),
        clock_origin: std::time::Instant::now(),
        lost: std::sync::Mutex::default(),
        revision_sequencer:
            crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new(),
        quest_catalogue: Some(std::sync::Arc::clone(owners.quest_catalogue)),
        quest_sessions: std::sync::Mutex::default(),
        chat: chat_intent::ChatRuntime::default(),
        npc_conversations: npc_command::NpcConversations::default(),
        // WHEEL-W1: the embedded Wheel ruleset; without it every Wheel load fails closed.
        wheel_ruleset: crate::wheel_gem_data::WheelGemData::embedded()
            .ok()
            .and_then(|data| data.wheel_ruleset().ok())
            .map(std::sync::Arc::new),
        wheel_sessions: std::sync::Mutex::default(),
        progression_sessions: std::sync::Mutex::default(),
        last_positions: last_position::LastPositions::default(),
        input_gate: input_gate::InputGate::default(),
        // PREM-WIRE-1: one Premium fence. The refresher pulls into the consumer the spell casts
        // and every other Premium gate read; a node without the Premium reads keeps its own.
        premium: premium_refresher(
            owners.root,
            owners
                .premium_coordinator
                .map_or_else(Default::default, |reads| reads.consumer()),
        ),
        premium_sessions: std::sync::Mutex::default(),
        fence_holders: FenceHolders::default(),
        spell_lane: crate::durability::spell_owner_commit::SpellLane::new(
            owners.world_id,
            owners.channel_id,
        ),
        loop_ground: loop_move::LoopGroundCache::default(),
    };
    if let Some(book) = owners.npc_talk {
        authority.chat.install_npc_talk(book);
    }
    // CHAT-2B: the World chat relay, when its key file is configured.
    if let Some(relay) = chat_relay::configured(
        *owners.world_id.as_bytes(),
        owners.root.chat_relay_connect_options(),
    ) {
        authority.chat.install_relay(std::sync::Arc::new(relay));
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
        quest_retries,
    );
    let mut serving = pin!(serving);
    let cycles_stop = CancellationToken::new();
    let mut cycles = pin!(authority.run_source_owner_cycles(&cycles_stop));
    let mut quiesce = pin!(async {
        drain.requested().await;
        authority.input_gate.quiesce().await;
    });
    let mut quiesced = false;
    // CHAR-POSITION-0 §3.2 (`CHARPOS0-RL-01`): the periodic last-position writes run on their own
    // deadlines, so no other cadence work delays them.
    let mut positions = pin!(authority.run_last_position_writes(shutdown));
    // CHAR-POSITION-0 §3.2: the shutdown write is polled with the seam and never holds its end.
    let mut flush = pin!(async {
        match owners.last_position_flush {
            Some(flush) => flush.serve(&authority).await,
            None => std::future::pending().await,
        }
    });
    // CHAT-2B: the relay listener; it parks once shutdown is requested.
    let mut relay = pin!(async {
        match authority.chat.relay() {
            Some(relay) => relay.run(&authority.chat, shutdown).await,
            None => std::future::pending().await,
        }
    });
    let mut serving_done = false;
    let mut cycles_done = false;
    let mut flush_done = false;
    poll_fn(|context| {
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
        if serving_done && cycles_done {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
    Ok(())
}
