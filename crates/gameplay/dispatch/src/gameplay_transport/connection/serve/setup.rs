//! Prepare and transmit the join snapshot, then initialize the live connection state.

use super::*;

struct LiveSetup {
    initial: snapshot::InitialSnapshot,
    items: snapshot::ItemSnapshot,
    sessions: snapshot::SessionSnapshot,
    world: snapshot::WorldSnapshot,
}

pub(super) async fn prepare<S, A>(
    stream: &mut S,
    admitted: &mut AdmittedSession,
    authority: &A,
    policy: LivenessPolicy,
    actor: ExactActorRef,
    baseline: WorldSpatialObservation,
    revisions: (u64, u64),
) -> Result<ServeState, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let (generation, revision) = revisions;
    // VIS-3: domain 1 goes first; with capability 6 it is composed after the item view below, so
    // that its objects carry their item handles under capability 4.
    let mut payloads = Vec::new();
    let mut initial = snapshot::initial(authority, actor, admitted, &mut payloads).await?;
    let mut items = snapshot::items(
        authority,
        actor,
        admitted,
        &mut payloads,
        baseline,
        revision,
    )
    .await?;
    if let Some((notice_revision, payload)) = initial.notices.take() {
        payloads.push(
            crate::gameplay_transport::item_view::ItemViewSnapshotDomain {
                domain_id: STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
                revision: notice_revision,
                snapshot_type: SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1,
                payload,
            },
        );
    }
    let sessions = snapshot::sessions(
        authority,
        actor,
        admitted,
        &mut payloads,
        &mut items.item_view,
    )
    .await?;
    let world = snapshot::world(
        authority,
        actor,
        admitted,
        &mut payloads,
        &mut items.item_view,
        items.selected_capabilities,
        baseline,
    )
    .await?;
    if admitted
        .continuity
        .selected_capabilities
        .contains(oteryn_protocol_oteryn::player_trade::CAPABILITY_PLAYER_TRADE_V1)
    {
        authority
            .player_trade_open(*admitted)
            .await
            .ok_or(ServeEnd::Disconnected)?;
        let (revision, state, _) = authority
            .player_trade_observation(admitted.game_session_id)
            .await
            .ok_or(ServeEnd::Disconnected)?;
        payloads.push(
            crate::gameplay_transport::item_view::ItemViewSnapshotDomain {
                domain_id: oteryn_protocol_oteryn::player_trade::STATE_DOMAIN_PLAYER_TRADE,
                revision,
                snapshot_type: 1,
                payload: oteryn_protocol_oteryn::player_trade::encode_state(&state)
                    .map_err(|_| ServeEnd::Disconnected)?,
            },
        );
    }
    let mut domains = payloads
        .iter()
        .map(|domain| DomainSnapshot {
            domain_id: domain.domain_id,
            revision: domain.revision,
            snapshot_type: domain.snapshot_type,
            payload: &domain.payload,
        })
        .collect();
    items.appearances.domain(&mut domains);
    domains.extend(tutorial_notices_transport::snapshot(admitted));
    let snapshot =
        encode_single_chunk_snapshot(generation, 1, admitted.continuity.server_sequence, &domains);
    let Ok(snapshot) = snapshot else {
        return Err(ServeEnd::Disconnected);
    };
    for frame in &snapshot {
        if write_frame(stream, frame).await.is_err() {
            return Err(ServeEnd::Disconnected);
        }
    }
    if let Some(trade) = payloads
        .iter()
        .find(|p| p.domain_id == oteryn_protocol_oteryn::player_trade::STATE_DOMAIN_PLAYER_TRADE)
    {
        admitted.continuity.player_trade_revision = trade.revision;
        authority.player_trade_delivered(admitted.game_session_id, trade.revision);
    }
    // r4122508665: only a fully, successfully transmitted snapshot (every frame, including
    // `SnapshotCommit`, above) may update continuity. The overlay is Channel-global (unlike
    // `spatial_revision`, this actor's own counter), so the reconnect fence (`resume.rs`,
    // r4122215795) must record what was actually confirmed delivered, never a value written
    // before transmission could still fail partway through.
    admitted.continuity.overlay_revision = initial.overlay_revision;
    if let Some((vitals_revision, _)) = &initial.vitals {
        admitted.continuity.vitals_revision = *vitals_revision;
    }
    if let Some((revision, state)) = &sessions.combat_sent {
        admitted.continuity.combat.snapshot_sent(*revision, *state);
    }
    live(
        authority,
        admitted,
        policy,
        actor,
        generation,
        revision,
        LiveSetup {
            initial,
            items,
            sessions,
            world,
        },
    )
    .await
}

async fn live<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &mut AdmittedSession,
    policy: LivenessPolicy,
    actor: ExactActorRef,
    generation: u64,
    revision: u64,
    setup: LiveSetup,
) -> Result<ServeState, ServeEnd> {
    let LiveSetup {
        initial,
        items,
        sessions,
        world,
    } = setup;
    let snapshot::InitialSnapshot {
        overlay_served,
        notices_selected,
        vitals,
        ..
    } = initial;
    let has_vitals = vitals.is_some();
    let snapshot::ItemSnapshot {
        party_view,
        item_view,
        visibility,
        appearances,
        selected_capabilities,
    } = items;
    let snapshot::SessionSnapshot {
        container_views,
        quest_log,
        chat_active,
        npc_active,
        combat_sent,
    } = sessions;
    let combat_active = combat_sent.is_some();
    let snapshot::WorldSnapshot {
        world_map,
        world_clock,
        world_clock_pacer,
        weather_sent,
    } = world;
    let sequence = admitted.continuity.server_sequence;
    let Some(commands) = crate::gameplay_transport::session_ingress::SessionCommandStream::open(
        authority.session_commands(),
        admitted,
    ) else {
        return Err(ServeEnd::Disconnected);
    };
    // NPC-ECON-1: only a connection that continues an earlier one of this GameSession can carry
    // an accepted original Bank command. A fresh admission (generation 1 at CommandId 1, FND-02)
    // is the session's first connection, so its first command never touches the Bank journal;
    // a same-session resume (FND-04B §16: a strictly newer generation) or any other carried
    // continuity reconciles it before replacement input.
    let reconcile_original_bank = admitted.continuity.connection_generation != ADMITTED_GENERATION
        || admitted.continuity.next_command_id != 1;
    let frames = FrameReader::default();
    let liveness = Liveness::new(policy);
    let mut cadence = tokio::time::interval_at(
        tokio::time::Instant::now() + policy.interval,
        policy.interval,
    );
    cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // SPELL-D8 §8.2: only an actor with `ACTOR_VITALS` has a Serene evaluation to run.
    let serene = has_vitals.then(|| {
        let mut serene = tokio::time::interval_at(
            tokio::time::Instant::now() + SERENE_EVALUATION,
            SERENE_EVALUATION,
        );
        serene.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        serene
    });
    // QUEST-LOG-WIRE-1: only a session with domain 16 looks for a changed quest copy.
    let quest_log_refresh = quest_log.is_some().then(|| {
        let mut refresh = tokio::time::interval_at(
            tokio::time::Instant::now() + QUEST_LOG_REFRESH,
            QUEST_LOG_REFRESH,
        );
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        refresh
    });
    // CHAT-WIRE-1: only a session with domain 12 drains its queued lines.
    let chat_refresh = chat_active.then(|| {
        let mut refresh =
            tokio::time::interval_at(tokio::time::Instant::now() + CHAT_REFRESH, CHAT_REFRESH);
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        refresh
    });
    // NPC-CMD-1a: a session with domain 7 follows its conversation (chat greeting, walking away).
    let npc_refresh = npc_active.then(|| {
        let mut refresh =
            tokio::time::interval_at(tokio::time::Instant::now() + CHAT_REFRESH, CHAT_REFRESH);
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        refresh
    });
    // The NPC domain 7 shows as the conversation partner; a fresh snapshot shows none.
    let npc_shown: Option<WireEntityRef> = None;
    // ATTACK-1b: only a session with domain 10 drives swings and follows its combat state.
    let combat_refresh = combat_active.then(|| {
        let mut refresh =
            tokio::time::interval_at(tokio::time::Instant::now() + COMBAT_REFRESH, COMBAT_REFRESH);
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        refresh
    });
    // VIS-3: only a session with capability 6 looks for entities that changed without its own
    // step.
    let visibility_refresh = visibility.is_some().then(|| {
        let mut refresh = tokio::time::interval_at(
            tokio::time::Instant::now() + VISIBILITY_REFRESH,
            VISIBILITY_REFRESH,
        );
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        refresh
    });
    // SPEED-1 (CONDITIONS-0 §4.3): the actor's pacing clock, this connection's one-step buffer
    // and the frames read while a step waits in it. Those frames run, in order, only after the
    // buffered step's result, because results commit in CommandId order (FND-02 §13); a step
    // among them arrived while the buffer was full and is refused. The buffer ends with the
    // connection, so a resumed connection starts with an empty one; the clock is the actor's
    // (CHASE-1, RANGED-0 §8): it starts from the actor's deadline and records to it, so a
    // replaced or resumed connection never steps sooner.
    let pacer = StepPacer::resumed(authority.actor_step_deadline(actor).await);
    // CHASE-1: this connection's claims on the actor's pacing clock live while it does.
    let step_holder = std::sync::Arc::new(());
    // STEP-PACE-WIRE-1: fixed for the connection; off, every step result keeps its former bytes.
    let step_wait_receipts = authority.step_wait_receipts();
    let buffered: Option<BufferedStep> = None;
    // Each held frame carries whether it arrived behind a buffered step (SPEED-1) rather than
    // behind a running logout (LOGOUT-RL-01).
    let held: std::collections::VecDeque<(Vec<u8>, bool)> = std::collections::VecDeque::new();
    // CHASE-1 (RANGED-0 §8): when the next chase step is checked, on the same pacing clock as a
    // client step; at most one is pending. Armed by a combat tick that holds a chasing target
    // and by a chase step that moved.
    let chase_due: Option<tokio::time::Instant> = None;
    // CHASE-1: a ready client frame was already read ahead of the due chase step; the next poll
    // runs that step first, so non-step traffic never postpones it indefinitely.
    let chase_yielded = false;
    Ok(ServeState {
        stash_rows: None,
        stash_revision: 0,
        admitted: *admitted,
        generation,
        actor,
        revision,
        sequence,
        selected_capabilities,
        overlay_served,
        notices_selected,
        party_view,
        item_view,
        visibility,
        appearances,
        container_views,
        quest_log,
        world_map,
        world_clock,
        world_clock_pacer,
        weather_sent,
        commands,
        reconcile_original_bank,
        frames,
        liveness,
        cadence,
        serene,
        quest_log_refresh,
        chat_refresh,
        npc_refresh,
        npc_shown,
        combat_refresh,
        visibility_refresh,
        pacer,
        step_holder,
        step_wait_receipts,
        buffered,
        held,
        chase_due,
        chase_yielded,
        policy,
    })
}
