//! Periodic state refreshes and chase progression.

use super::*;
use oteryn_protocol_oteryn::chat::{line_extended, private_channel_extended};

pub(super) async fn serene<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    if session.selected_capabilities.contains(290) {
        authority.cast_refresh(session.admitted).await;
    }
    let changed = authority
        .tick_vitals(session.actor, session.admitted.game_session_id)
        .await;
    if let Some(catalog) = &mut session.spell_catalog {
        catalog
            .refresh(
                stream,
                authority,
                session.actor,
                &mut session.admitted,
                &mut session.sequence,
            )
            .await?;
    }
    sounds(stream, authority, session).await?;
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
    if session.cast_chat_revision > 0 {
        for (delta_type, payload) in authority.cast_live_chat(session.admitted).await {
            let revision = session
                .cast_chat_revision
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let sequence = session
                .sequence
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let frame = encode_state_delta(
                session.generation,
                sequence,
                103,
                session.cast_chat_revision,
                revision,
                delta_type,
                &payload,
            )
            .map_err(|_| ServeEnd::Disconnected)?;
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
            session.sequence = sequence;
            session.admitted.continuity.server_sequence = sequence;
            session.cast_chat_revision = revision;
        }
    }
    channel_list(stream, authority, session).await?;
    typing(stream, authority, session).await?;
    super::vip_presence::refresh(stream, authority, session).await?;
    private_echoes(stream, authority, session).await?;
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
        if session
            .selected_capabilities
            .contains(line_extended::CAPABILITY_CHAT_LINE_SEQUENCE_V1)
        {
            let payload = encode_chat_line(&line).map_err(|_| ServeEnd::Disconnected)?;
            write_line(stream, session, payload).await?;
        } else {
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
        }
        if session.selected_capabilities.contains(290) {
            authority.cast_heard(session.admitted, &line).await;
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

/// WU26: one domain 12 line delta (type 1) of an encoded line. With capability 496 the line
/// carries field 13, the delta's own new revision, so it strictly increases per GameSession.
async fn write_line<S>(
    stream: &mut S,
    session: &mut ServeState,
    mut payload: Vec<u8>,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    if session
        .selected_capabilities
        .contains(line_extended::CAPABILITY_CHAT_LINE_SEQUENCE_V1)
    {
        let line_sequence = session
            .admitted
            .continuity
            .chat_revision
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        line_extended::append_line_sequence(&mut payload, line_sequence)
            .map_err(|_| ServeEnd::Disconnected)?;
    }
    let Some((delta_sequence, frame, chat_revision)) = chat_delta(
        session.generation,
        session.sequence,
        session.admitted.continuity.chat_revision,
        DELTA_TYPE_CHAT_LINE_V1,
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

/// WU26 capability 493: the echoes of the session's accepted private lines, oldest first, at most
/// one egress bound per refresh.
async fn private_echoes<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    if !session
        .selected_capabilities
        .contains(line_extended::CAPABILITY_CHAT_PRIVATE_ECHO_V1)
    {
        return Ok(());
    }
    for _ in 0..EGRESS_MAX_LINES {
        let Some(echo) = authority.chat_private_echo_next(session.admitted).await else {
            break;
        };
        let payload =
            line_extended::encode_private_echo(&echo).map_err(|_| ServeEnd::Disconnected)?;
        write_line(stream, session, payload).await?;
    }
    Ok(())
}

/// S19: queued typing changes share the CHAT cadence and revision stream.
async fn typing<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    use oteryn_protocol_oteryn::player_typing;
    if !session
        .selected_capabilities
        .contains(player_typing::CAPABILITY_PLAYER_TYPING_V1)
    {
        return Ok(());
    }
    for _ in 0..EGRESS_MAX_LINES {
        let Some(update) = authority.player_typing_next(session.admitted).await else {
            break;
        };
        let payload = player_typing::encode_update(update);
        let Some((sequence, frame, revision)) = chat_delta(
            session.generation,
            session.sequence,
            session.admitted.continuity.chat_revision,
            player_typing::DELTA_TYPE_PLAYER_TYPING_V1,
            &payload,
        ) else {
            return Err(ServeEnd::Disconnected);
        };
        session.sequence = sequence;
        session.admitted.continuity.server_sequence = sequence;
        session.admitted.continuity.chat_revision = revision;
        if write_frame(stream, &frame).await.is_err() {
            return Err(ServeEnd::Disconnected);
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
    if session
        .selected_capabilities
        .contains(private_channel_extended::CAPABILITY_CHAT_JOINED_CHANNELS_V1)
        || session
            .selected_capabilities
            .contains(private_channel_extended::CAPABILITY_CHAT_CHANNEL_PARTICIPANTS_V1)
    {
        return channel_list_extended(stream, authority, session).await;
    }
    let Some(list) = authority.chat_channel_list_bound(session.admitted).await else {
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

/// WU26 capabilities 494 and 495: the list delta with fields 4 and 5 when the list or its
/// extension changed, then one line kind 8 per member notice since the previous read.
async fn channel_list_extended<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    use oteryn_protocol_oteryn::chat::private_channel;
    let (list, notices) = authority
        .chat_channel_list_extended_bound(session.admitted)
        .await;
    if let Some((list, extension)) = list {
        let payload = private_channel_extended::encode_channel_list_extended(&list, &extension)
            .map_err(|_| ServeEnd::Disconnected)?;
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
    }
    for notice in notices {
        let payload =
            line_extended::encode_channel_notice(&notice).map_err(|_| ServeEnd::Disconnected)?;
        write_line(stream, session, payload).await?;
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
    sounds(stream, authority, session).await?;
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
        session.container_views.as_mut(),
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
    sounds(stream, authority, session).await?;
    if let Some(view) = session.item_view.as_mut()
        && view.inventory_refresh_pending
    {
        item_use_view_refresh::refresh(
            stream,
            authority,
            session.actor,
            &mut session.admitted,
            view,
            session.container_views.as_mut(),
            &mut session.sequence,
        )
        .await
        .ok_or(ServeEnd::Disconnected)?;
    }
    refresh_newhaven_inventory(
        stream,
        authority,
        session.actor,
        &mut session.admitted,
        session.item_view.as_mut(),
        session.container_views.as_mut(),
        &mut session.sequence,
    )
    .await
    .ok_or(ServeEnd::Disconnected)?;
    player_trade::refresh(stream, authority, session).await?;
    cyclopedia_tracker::refresh(stream, authority, session).await?;
    // Crystal's tile notifications refresh every observer, including an idle one.
    // Use the same bounded connection refresh already used after local item commits.
    if session
        .selected_capabilities
        .contains(oteryn_protocol_oteryn::browse_field::CAPABILITY_BROWSE_FIELD_V1)
        && let (Some(views), Some(view)) =
            (session.container_views.as_mut(), session.item_view.as_mut())
    {
        let refreshed = crate::gameplay_transport::container_view::refresh::refresh(
            authority,
            session.actor,
            session.admitted.item_fence,
            views,
            view,
        )
        .await
        .map_err(|_| ServeEnd::Disconnected)?;
        session.admitted.continuity.item_view = view.continuity();
        if let Some(delta) = refreshed {
            let (sequence, frame) = item_view_delta(session.generation, session.sequence, &delta)
                .ok_or(ServeEnd::Disconnected)?;
            session.sequence = sequence;
            session.admitted.continuity.server_sequence = sequence;
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
        }
    }
    // VIS-3: entities that moved, entered or left without this actor's step.
    let Some(view) = session.visibility.as_mut() else {
        return Ok(());
    };
    let Some(channel) = crate::gameplay_transport::entity_names::observe(
        authority,
        session.actor,
        session.admitted.continuity.selected_capabilities,
    )
    .await
    else {
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
    party_skull::refresh(stream, authority, session).await?;
    // MAP-WIRE-2: tiles in view changed by others (an item dropped, a door opened).
    if let (Some(view), Some(items)) = (session.world_map.as_mut(), session.item_view.as_mut()) {
        let update = world_map_frames(
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
        if let Some((next_sequence, frames)) = update {
            session.sequence = next_sequence;
            session.admitted.continuity.server_sequence = session.sequence;
            session.admitted.continuity.world_map_revision = view.revision();
            for frame in frames {
                if write_frame(stream, &frame).await.is_err() {
                    return Err(ServeEnd::Disconnected);
                }
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
pub(super) async fn sounds<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    use oteryn_protocol_oteryn::presentation as wire;
    let presentation_selected = session
        .selected_capabilities
        .contains(wire::CAPABILITY_PRESENTATION_V1);
    let ambient_selected = session
        .selected_capabilities
        .contains(wire::CAPABILITY_AMBIENT_SOUND_V1);
    let item_selected = session
        .selected_capabilities
        .contains(wire::CAPABILITY_ITEM_EFFECTS_V1);
    if !presentation_selected && !ambient_selected && !item_selected {
        return Ok(());
    }
    let Some((cursor, mut events)) = authority
        .observe_sounds(
            session.actor,
            session.admitted.game_session_id,
            session.sound_cursor,
        )
        .await
    else {
        return Ok(());
    };
    if ambient_selected
        && let Some(cue_id) = events.iter().find_map(|event| match event.kind {
            wire::PresentationEventKind::AmbientSound { cue_id } => Some(cue_id),
            _ => None,
        })
    {
        ambient_sound(stream, session, cue_id).await?;
    }

    if session.selected_capabilities.contains(290) {
        authority.cast_effects(session.admitted, &events).await;
    }
    let item_events: Vec<_> = events
        .iter()
        .copied()
        .filter(|event| {
            item_selected && matches!(event.kind, wire::PresentationEventKind::MagicEffect { .. })
        })
        .collect();
    events.retain(|event| {
        presentation_selected
            && !matches!(event.kind, wire::PresentationEventKind::AmbientSound { .. })
            && !(item_selected
                && matches!(event.kind, wire::PresentationEventKind::MagicEffect { .. }))
    });
    for (index, event) in events.iter_mut().enumerate() {
        event.decision_ordinal = u32::try_from(index + 1).map_err(|_| ServeEnd::Disconnected)?;
    }
    if !events.is_empty() {
        let payload =
            wire::encode_presentation_batch(&events).map_err(|_| ServeEnd::Disconnected)?;
        let revision = session
            .admitted
            .continuity
            .presentation_revision
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        let sequence = session
            .sequence
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        let frame = encode_state_delta(
            session.generation,
            sequence,
            wire::STATE_DOMAIN_WORLD_PRESENTATION,
            session.admitted.continuity.presentation_revision,
            revision,
            wire::DELTA_TYPE_WORLD_PRESENTATION_BATCH_V1,
            &payload,
        )
        .map_err(|_| ServeEnd::Disconnected)?;
        write_frame(stream, &frame)
            .await
            .map_err(|_| ServeEnd::Disconnected)?;
        session.sequence = sequence;
        session.admitted.continuity.server_sequence = sequence;
        session.admitted.continuity.presentation_revision = revision;
    }
    if !item_events.is_empty() {
        item_effects(stream, session, &item_events).await?;
    }
    session.sound_cursor = cursor;
    Ok(())
}

/// One private replacement, including the silence sent before entering House custody.
pub(super) async fn ambient_sound<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    session: &mut ServeState,
    cue_id: u32,
) -> Result<(), ServeEnd> {
    use oteryn_protocol_oteryn::presentation as wire;
    let event = wire::PresentationEvent {
        decision_ordinal: 1,
        emission_ordinal: 1,
        kind: wire::PresentationEventKind::AmbientSound { cue_id },
    };
    let payload = wire::encode_presentation_batch(&[event]).map_err(|_| ServeEnd::Disconnected)?;
    let revision = session
        .ambient_revision
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let sequence = session
        .sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let frame = encode_state_delta(
        session.generation,
        sequence,
        wire::STATE_DOMAIN_AMBIENT_SOUND,
        session.ambient_revision,
        revision,
        1,
        &payload,
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    write_frame(stream, &frame)
        .await
        .map_err(|_| ServeEnd::Disconnected)?;
    session.ambient_revision = revision;
    session.sequence = sequence;
    session.admitted.continuity.server_sequence = sequence;
    Ok(())
}

async fn item_effects<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    session: &mut ServeState,
    events: &[oteryn_protocol_oteryn::presentation::PresentationEvent],
) -> Result<(), ServeEnd> {
    use oteryn_protocol_oteryn::presentation as wire;
    let payload = wire::encode_item_effect_batch(events).map_err(|_| ServeEnd::Disconnected)?;
    let revision = session
        .item_effects_revision
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let sequence = session
        .sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let frame = encode_state_delta(
        session.generation,
        sequence,
        wire::STATE_DOMAIN_ITEM_EFFECTS,
        session.item_effects_revision,
        revision,
        1,
        &payload,
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    write_frame(stream, &frame)
        .await
        .map_err(|_| ServeEnd::Disconnected)?;
    session.sequence = sequence;
    session.admitted.continuity.server_sequence = sequence;
    session.item_effects_revision = revision;
    Ok(())
}
