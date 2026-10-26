//! Periodic state refreshes and chase progression.

use super::*;

pub(super) async fn serene<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let changed = authority
        .tick_vitals(session.actor, session.admitted.game_session_id)
        .await;
    let current = authority
        .observe_vitals(session.actor, session.admitted.game_session_id)
        .await;
    let Some((to, value)) = current
        .filter(|(observed_revision, _)| {
            *observed_revision > session.admitted.continuity.vitals_revision
        })
        .or(changed.filter(|(observed_revision, _)| {
            *observed_revision > session.admitted.continuity.vitals_revision
        }))
    else {
        return Ok(());
    };
    let Some((delta_sequence, delta)) = vitals_delta(
        session.generation,
        session.sequence,
        session.admitted.continuity.vitals_revision,
        to,
        &value,
    ) else {
        return Err(ServeEnd::Disconnected);
    };
    session.sequence = delta_sequence;
    session.admitted.continuity.server_sequence = session.sequence;
    if write_frame(stream, &delta).await.is_err() {
        return Err(ServeEnd::Disconnected);
    }
    session.admitted.continuity.vitals_revision = to;
    Ok(())
}

pub(super) async fn quest_log<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    // QUEST-LOG-WIRE-1: a committed receipt that changes what domain 16 shows sends
    // a delta; an unchanged or unavailable copy sends nothing.
    let Some(state) = session.quest_log.as_mut() else {
        return Ok(());
    };
    let observation = authority
        .observe_quest_log(
            session.actor,
            session.admitted.game_session_id,
            state.version(),
        )
        .await;
    let Ok(changed) = state.refresh(&mut session.admitted.continuity.quest_log, observation) else {
        return Err(ServeEnd::Disconnected);
    };
    let Some(domain) = changed else {
        return Ok(());
    };
    let Some((delta_sequence, frame)) =
        quest_log_delta(session.generation, session.sequence, &domain)
    else {
        return Err(ServeEnd::Disconnected);
    };
    session.sequence = delta_sequence;
    session.admitted.continuity.server_sequence = session.sequence;
    if write_frame(stream, &frame).await.is_err() {
        return Err(ServeEnd::Disconnected);
    }
    Ok(())
}

pub(super) async fn chat<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    channel_list(stream, authority, session).await?;
    // CHAT-WIRE-1: every heard line queued since the last tick is one domain 12
    // delta; an empty queue sends nothing.
    // At most one full queue per refresh: lines that keep arriving cannot hold the
    // connection here, and the rest waits for the next tick.
    let mut owner_reads = EGRESS_MAX_LINES;
    for _ in 0..EGRESS_MAX_LINES {
        let Some(line) = authority
            .chat_next_bound(session.admitted, &mut owner_reads)
            .await
        else {
            break;
        };
        let Some((delta_sequence, frame, chat_revision)) = chat_line_delta(
            session.generation,
            session.sequence,
            session.admitted.continuity.chat_revision,
            &line,
        ) else {
            return Err(ServeEnd::Disconnected);
        };
        session.sequence = delta_sequence;
        session.admitted.continuity.server_sequence = session.sequence;
        session.admitted.continuity.chat_revision = chat_revision;
        if write_frame(stream, &frame).await.is_err() {
            return Err(ServeEnd::Disconnected);
        }
        // A marker takes no slot, so only a real line frees one.
        if !matches!(line, ChatLine::Dropped) {
            authority
                .chat_delivered(session.admitted.game_session_id)
                .await;
        }
    }
    Ok(())
}

/// CHAT-PM-CHANNELS-1: a changed channel list is one domain 12 delta type 3, with capability 80
/// only.
pub(super) async fn channel_list<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    use oteryn_protocol_oteryn::chat::private_channel;
    if !session
        .selected_capabilities
        .contains(private_channel::CAPABILITY_CHAT_PRIVATE_CHANNEL_V1)
    {
        return Ok(());
    }
    let Some(list) = authority
        .chat_channel_list(session.admitted.game_session_id)
        .await
    else {
        return Ok(());
    };
    let payload =
        private_channel::encode_channel_list(&list).map_err(|_| ServeEnd::Disconnected)?;
    let Some((delta_sequence, frame, chat_revision)) = chat_delta(
        session.generation,
        session.sequence,
        session.admitted.continuity.chat_revision,
        private_channel::DELTA_TYPE_CHAT_CHANNEL_LIST_V1,
        &payload,
    ) else {
        return Err(ServeEnd::Disconnected);
    };
    session.sequence = delta_sequence;
    session.admitted.continuity.server_sequence = session.sequence;
    session.admitted.continuity.chat_revision = chat_revision;
    if write_frame(stream, &frame).await.is_err() {
        return Err(ServeEnd::Disconnected);
    }
    Ok(())
}

pub(super) async fn npc<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let Some(update) = authority
        .npc_follow(
            session.actor,
            session.admitted.game_session_id,
            session.npc_shown,
        )
        .await
    else {
        return Ok(());
    };
    let Some(update_frames) = npc_update_frames(
        session.generation,
        &mut session.sequence,
        &mut session.admitted.continuity.npc,
        &update,
        &mut session.npc_shown,
    ) else {
        return Err(ServeEnd::Disconnected);
    };
    session.admitted.continuity.server_sequence = session.sequence;
    for frame in update_frames {
        if write_frame(stream, &frame).await.is_err() {
            return Err(ServeEnd::Disconnected);
        }
    }
    Ok(())
}

pub(super) async fn combat<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    // ATTACK-1b: due swings run; a changed domain 10 state sends a delta.
    pvp_expert::refresh(stream, authority, session).await?;
    let Some(state) = authority
        .tick_combat(session.actor, session.admitted.game_session_id)
        .await
    else {
        return Ok(());
    };
    if session.chase_due.is_none()
        && state.target.is_some()
        && state.modes.chase == ChaseMode::Chase
    {
        let now = tokio::time::Instant::now();
        session.chase_due = Some(session.pacer.wait_until(now).unwrap_or(now));
    }
    let Some(change) =
        session
            .admitted
            .continuity
            .combat
            .delta(session.generation, session.sequence, state)
    else {
        return Err(ServeEnd::Disconnected);
    };
    let Some((delta_sequence, frame)) = change else {
        return Ok(());
    };
    session.sequence = delta_sequence;
    session.admitted.continuity.server_sequence = session.sequence;
    if write_frame(stream, &frame).await.is_err() {
        return Err(ServeEnd::Disconnected);
    }
    Ok(())
}

pub(super) async fn chase<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    // CHASE-1: a client step waiting in the buffer wins over the chase.
    session.chase_due = None;
    session.chase_yielded = false;
    if session.buffered.is_some() {
        return Ok(());
    }
    if let Some(due) = begin_actor_step(
        &mut session.pacer,
        authority,
        session.actor,
        &session.step_holder,
    )
    .await
    {
        session.chase_due = Some(due);
        return Ok(());
    }
    let step = authority
        .chase_step(session.actor, session.admitted.game_session_id)
        .await;
    let duration = step.as_ref().and_then(|(_, duration)| *duration);
    record_actor_step(
        &mut session.pacer,
        authority,
        session.actor,
        &session.step_holder,
        duration,
    )
    .await;
    let Some((outcome, _)) = step else {
        // RANGED0-RL-03: a search the Channel budget refused retries in the next
        // owner window, not at the next combat tick.
        if authority
            .chase_search_waiting(session.actor, session.admitted.game_session_id)
            .await
        {
            session.chase_due = Some(tokio::time::Instant::now() + CHASE_SEARCH_RETRY);
        }
        return Ok(());
    };
    let Some(observation) = outcome.moved_to else {
        return Ok(());
    };
    if moved_step_frames(
        stream,
        authority,
        session.actor,
        &mut session.admitted,
        session.visibility.as_mut(),
        session.item_view.as_mut(),
        session.world_map.as_mut(),
        session.generation,
        &mut session.sequence,
        &mut session.revision,
        &mut session.appearances,
        observation,
    )
    .await
    .is_none()
    {
        return Err(ServeEnd::Disconnected);
    }
    let now = tokio::time::Instant::now();
    session.chase_due = Some(session.pacer.wait_until(now).unwrap_or(now));
    Ok(())
}

pub(super) async fn visibility<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    player_trade::refresh(stream, authority, session).await?;
    // VIS-3: entities that moved, entered or left without this actor's step.
    let Some(view) = session.visibility.as_mut() else {
        return Ok(());
    };
    let Some(channel) = authority.observe_visible_entities(session.actor).await else {
        return Err(ServeEnd::Disconnected);
    };
    let Some((to_sequence, to_revision)) = session
        .appearances
        .refresh(
            stream,
            authority,
            session.actor,
            &mut session.admitted,
            view,
            &channel,
            session.item_view.as_mut(),
            session.sequence,
            session.revision,
        )
        .await
    else {
        return Err(ServeEnd::Disconnected);
    };
    session.sequence = to_sequence;
    session.revision = to_revision;
    session.admitted.continuity.spatial_revision = session.revision;
    // MAP-WIRE-2: tiles in view changed by others (an item dropped, a door opened).
    if let (Some(view), Some(items)) = (session.world_map.as_mut(), session.item_view.as_mut()) {
        let update = world_map_delta(
            authority,
            session.actor,
            session.generation,
            session.sequence,
            view,
            items,
        )
        .await;
        session.admitted.continuity.item_view = items.continuity();
        let Some(update) = update else {
            return Err(ServeEnd::Disconnected);
        };
        if let Some((delta_sequence, frame)) = update {
            session.sequence = delta_sequence;
            session.admitted.continuity.server_sequence = session.sequence;
            session.admitted.continuity.world_map_revision = view.revision();
            if write_frame(stream, &frame).await.is_err() {
                return Err(ServeEnd::Disconnected);
            }
        }
    }
    // CLOCK-1 (§11.3): a due domain 18 delta, at most one per 10,000 ms.
    if session
        .selected_capabilities
        .contains(CAPABILITY_WORLD_LIGHT_V1)
    {
        let Some(now) = authority.world_clock_utc_ms() else {
            return Err(ServeEnd::Disconnected);
        };
        if let Some(reading) = session.world_clock_pacer.due(&session.world_clock, now) {
            let Some((delta_sequence, frame, to)) = world_clock_delta(
                session.generation,
                session.sequence,
                session.admitted.continuity.world_clock_revision,
                reading,
            ) else {
                return Err(ServeEnd::Disconnected);
            };
            session.sequence = delta_sequence;
            session.admitted.continuity.server_sequence = session.sequence;
            session.admitted.continuity.world_clock_revision = to;
            if write_frame(stream, &frame).await.is_err() {
                return Err(ServeEnd::Disconnected);
            }
        }
    }
    // WEATHER-1: a full-replacement domain 22 delta only when the observation
    // changed (a new absolute slot, or a lost or restored binding). Visual only: no
    // gameplay owner is read or written.
    if let (Some(sent), Some(here)) = (session.weather_sent, authority.observe(session.actor).await)
    {
        let observed = observe_weather(authority, here.actor_position);
        if observed != sent {
            let Some((delta_sequence, frame, to)) = world_weather_delta(
                session.generation,
                session.sequence,
                session.admitted.continuity.world_weather_revision,
                observed,
            ) else {
                return Err(ServeEnd::Disconnected);
            };
            session.sequence = delta_sequence;
            session.admitted.continuity.server_sequence = session.sequence;
            session.admitted.continuity.world_weather_revision = to;
            if write_frame(stream, &frame).await.is_err() {
                return Err(ServeEnd::Disconnected);
            }
            session.weather_sent = Some(observed);
        }
    }
    Ok(())
}
