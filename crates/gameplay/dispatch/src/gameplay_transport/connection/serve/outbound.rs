//! Terminalize and send the result before ordered domain updates and relocation.

use super::*;

pub(super) async fn finish<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    dispatch: Dispatch,
    result: Vec<u8>,
    outcome: CommandOutcome,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    if session
        .commands
        .terminalize(command.command_id, &result)
        .is_none()
    {
        return Err(ServeEnd::Disconnected);
    }
    session.admitted.continuity.next_command_id = session.commands.next_command_id();
    if let Dispatch::Logout(LogoutResult::Accepted) = dispatch {
        // §1.6: the release committed before this result. No command is read any more and
        // the transport closes cleanly; a failed write is no control loss either. A second
        // logout read before this result is still the `LOGOUT-RL-01` protocol error, but the
        // committed release keeps this a logout end.
        if write_frame(stream, &result).await.is_ok()
            && (!outcome.second_logout
                || send_error(
                    stream,
                    FoundationProtocolError::TooManyOutstandingCommands,
                    session.admitted.continuity.connection_generation,
                )
                .await
                .is_ok())
        {
            let _ = tokio::time::timeout(
                session.policy.interval,
                tokio::io::AsyncWriteExt::shutdown(stream),
            )
            .await;
        }
        return Err(ServeEnd::LoggedOut);
    }
    if write_frame(stream, &result).await.is_err() {
        return Err(ServeEnd::Disconnected);
    }
    if matches!(dispatch, Dispatch::Logout(_)) {
        if outcome.second_logout {
            return Err(close(
                stream,
                session.admitted,
                FoundationProtocolError::TooManyOutstandingCommands,
            )
            .await);
        }
        if outcome.lost_while_logging_out {
            return Err(ServeEnd::Disconnected);
        }
    }
    emit(
        stream,
        authority,
        session,
        command,
        dispatch,
        outcome.npc_relocation,
        outcome.training_logged_out,
    )
    .await
}

async fn emit<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    dispatch: Dispatch,
    npc_relocation: Option<WorldSpatialObservation>,
    training_logged_out: bool,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    match dispatch {
        Dispatch::Step(outcome, _) => {
            effects::step(stream, authority, session, outcome).await?;
        }
        Dispatch::Use(outcome) => {
            use_effects::use_effects(stream, authority, session, command, outcome).await?;
        }
        Dispatch::Spell(outcome) => {
            effects::spell(stream, authority, session, outcome).await?;
        }
        Dispatch::Timed(_, deltas) => {
            for delta in deltas {
                let Some((delta_sequence, frame)) =
                    item_view_delta(session.generation, session.sequence, &delta)
                else {
                    return Err(ServeEnd::Disconnected);
                };
                session.sequence = delta_sequence;
                session.admitted.continuity.server_sequence = session.sequence;
                if write_frame(stream, &frame).await.is_err() {
                    return Err(ServeEnd::Disconnected);
                }
            }
        }
        Dispatch::UseItem(_, Some(delta)) | Dispatch::ContainerView(_, Some(delta)) => {
            let Some((delta_sequence, frame)) =
                item_view_delta(session.generation, session.sequence, &delta)
            else {
                return Err(ServeEnd::Disconnected);
            };
            session.sequence = delta_sequence;
            session.admitted.continuity.server_sequence = session.sequence;
            if write_frame(stream, &frame).await.is_err() {
                return Err(ServeEnd::Disconnected);
            }
        }
        Dispatch::Depot(reply) => {
            if reply.outcome == oteryn_protocol_oteryn::depot::DepotOutcome::Moved {
                let Some(view) = session.item_view.as_mut() else {
                    return Err(ServeEnd::Disconnected);
                };
                if item_use_view_refresh::refresh(
                    stream,
                    authority,
                    session.actor,
                    &mut session.admitted,
                    view,
                    session.container_views.as_mut(),
                    &mut session.sequence,
                )
                .await
                .is_none()
                {
                    return Err(ServeEnd::Disconnected);
                }
            }
        }
        Dispatch::ItemMove(outcome, deltas, _) => {
            effects::item_move(stream, authority, session, outcome, deltas).await?;
        }
        Dispatch::QuestLog(domain) => {
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
        }
        // CHAT-WIRE-1: the lines reach the hearers' domain 12 on their own ticks. CHAT-2B: an
        // opened or closed room follows the result as one domain 12 delta type 2.
        Dispatch::Chat(_, spell) => {
            if let Some(outcome) = spell {
                effects::spell(stream, authority, session, outcome).await?;
            }
            if let Some(rooms) = authority
                .chat_rooms_changed(session.admitted.game_session_id)
                .await
            {
                let Some((delta_sequence, frame, chat_revision)) = chat_delta(
                    session.generation,
                    session.sequence,
                    session.admitted.continuity.chat_revision,
                    DELTA_TYPE_CHAT_ROOMS_V1,
                    &encode_chat_rooms(rooms),
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
        }
        // NPC-CMD-1a: the whole domain 7 payload, then the domain 8 window (NPC-CMD-1b-read),
        // follow the result, to the speaker alone.
        Dispatch::NpcTrade(_, update, deltas) => {
            effects::npc_trade(stream, session, update, deltas).await?;
        }
        Dispatch::Npc(_, update, _) => {
            effects::npc(stream, session, update).await?;
        }
        Dispatch::Attack(_) => {
            // The changed target or modes reach domain 10 right after the result.
            let Some(state) = authority
                .observe_combat_state(session.actor, session.admitted.game_session_id)
                .await
            else {
                return Ok(());
            };
            let Some(change) = session.admitted.continuity.combat.delta(
                session.generation,
                session.sequence,
                state,
            ) else {
                return Err(ServeEnd::Disconnected);
            };
            if let Some((delta_sequence, frame)) = change {
                session.sequence = delta_sequence;
                session.admitted.continuity.server_sequence = session.sequence;
                if write_frame(stream, &frame).await.is_err() {
                    return Err(ServeEnd::Disconnected);
                }
            }
        }
        Dispatch::Guild(_) if command.command_type == 73 && command.payload.first() == Some(&4) => {
            if let Some(view) = session.item_view.as_mut()
                && item_use_view_refresh::refresh(
                    stream,
                    authority,
                    session.actor,
                    &mut session.admitted,
                    view,
                    session.container_views.as_mut(),
                    &mut session.sequence,
                )
                .await
                .is_none()
            {
                return Err(ServeEnd::Disconnected);
            }
        }
        Dispatch::UseItem(_, None)
        | Dispatch::ContainerView(_, None)
        | Dispatch::Look(..)
        | Dispatch::Achievements(_)
        | Dispatch::Highscores(_)
        | Dispatch::Guild(_)
        | Dispatch::Reply(..)
        | Dispatch::Vip(_)
        | Dispatch::Appearance(_)
        | Dispatch::Training(_)
        | Dispatch::Logout(_)
        | Dispatch::House(_)
        | Dispatch::Unregistered => {}
    }
    if training_logged_out {
        return Err(ServeEnd::LoggedOut);
    }
    if newhaven_travel_transport::emit_relocation(
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
        npc_relocation,
    )
    .await
    .is_none()
    {
        return Err(ServeEnd::Disconnected);
    }
    Ok(())
}
