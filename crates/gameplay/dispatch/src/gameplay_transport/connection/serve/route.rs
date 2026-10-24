//! Execute command routing after ingress, input and capability gates.

use super::*;

pub(super) enum StepAction {
    Wait(tokio::time::Instant),
    Dispatch(Dispatch),
}

pub(super) async fn step<A: FreshAdmissionAuthority>(
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    arrived_while_buffered: bool,
) -> StepAction {
    StepAction::Dispatch(
        // SPEED-1 (CONDITIONS-0 §4.3): every player step is paced. An early step waits in the
        // one-step buffer, without a result yet; one that arrived while the buffer was full is
        // refused and nothing moves.
        match decode_step_intent(command.payload) {
            Err(_) => Dispatch::Step(StepOutcome::rejected(), 0),
            // STEP-PACE-WIRE-1: the receipt is the time left until the pacing clock is due,
            // after the buffered step ran; none when that step did not move.
            Ok(_) if arrived_while_buffered => {
                let now = tokio::time::Instant::now();
                let left = session
                    .pacer
                    .wait_until(now)
                    .map(|due| due.duration_since(now));
                Dispatch::Step(StepOutcome::too_early(), too_early_step_wait_ms(left))
            }
            Ok(direction) => {
                // CHASE-1: client intent wins; the pending chase step and its route drop.
                session.chase_due = None;
                session.chase_yielded = false;
                authority
                    .yield_chase_to_client(session.actor, session.admitted.game_session_id)
                    .await;
                let now = tokio::time::Instant::now();
                let wait = match session.pacer.wait_until(now) {
                    Some(due) => Some(due),
                    None => {
                        begin_actor_step(
                            &mut session.pacer,
                            authority,
                            session.actor,
                            &session.step_holder,
                        )
                        .await
                    }
                };
                if let Some(due) = wait {
                    return StepAction::Wait(due);
                }
                let (outcome, duration) = authority
                    .paced_step(session.actor, session.admitted.game_session_id, direction)
                    .await;
                // The clock starts when the step committed, so a slow owner turn never
                // shortens the next step's wait.
                record_actor_step(
                    &mut session.pacer,
                    authority,
                    session.actor,
                    &session.step_holder,
                    duration,
                )
                .await;
                // STEP-PACE-WIRE-1: a moved step's receipt is the duration just recorded.
                let wait = if outcome.disposition == StepDisposition::Moved {
                    moved_step_wait_ms(duration)
                } else {
                    0
                };
                Dispatch::Step(outcome, wait)
            }
        },
    )
}

async fn logout<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    outcome: &mut CommandOutcome,
) -> Result<Dispatch, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    Ok(
        // LOGOUT-WIRE-1 (§1.6): a non-empty intent is REJECTED with an empty payload before
        // any authority call. While the logout runs, frames are read and held, in order, for
        // after its result (LOGOUT-RL-01): a second logout among them is a protocol error.
        if decode_logout_intent(command.payload).is_err() {
            Dispatch::Unregistered
        } else {
            // A paced step can already have held the frames that follow this logout; a
            // second logout among them was read before this result too (#2127 Codex P2
            // 4279788741).
            outcome.second_logout = session
                .held
                .iter()
                .any(|(frame, _)| is_logout_command(frame, session.generation));
            let mut logout = std::pin::pin!(authority.logout(session.admitted));
            let reply = loop {
                let reading = !outcome.lost_while_logging_out
                    && session.held.len() < MAX_FRAMES_HELD_BEHIND_A_BUFFERED_STEP;
                let mut read = std::pin::pin!(session.frames.next(stream));
                let next = std::future::poll_fn(|context| {
                    if let std::task::Poll::Ready(reply) = logout.as_mut().poll(context) {
                        return std::task::Poll::Ready(Ok(reply));
                    }
                    if reading && let std::task::Poll::Ready(read) = read.as_mut().poll(context) {
                        return std::task::Poll::Ready(Err(read));
                    }
                    std::task::Poll::Pending
                })
                .await;
                match next {
                    Ok(reply) => break reply,
                    Err(Ok(frame)) => {
                        outcome.second_logout |= is_logout_command(&frame, session.generation);
                        session.held.push_back((frame, false));
                    }
                    Err(Err(_)) => outcome.lost_while_logging_out = true,
                }
            };
            match reply {
                LogoutReply::Result(result) => Dispatch::Logout(result),
                // §1.6 unknown outcome: no result; the loss lifecycle reconciles the row.
                LogoutReply::Unknown => {
                    return Err(ServeEnd::Disconnected);
                }
            }
        },
    )
}

pub(super) async fn execute<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    outcome: &mut CommandOutcome,
) -> Result<Dispatch, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    Ok(if matches!(command.command_type, 66 | 67) {
        let Some(result) = crate::gameplay_transport::loop_training::wire::dispatch(
            authority,
            session.admitted,
            UseCommand {
                game_session_id: session.admitted.game_session_id,
                command_id: command.command_id,
                item_fence: session.admitted.item_fence,
            },
            command.command_type,
            command.payload,
            session.item_view.as_ref(),
        )
        .await
        else {
            return Err(ServeEnd::ControlLost);
        };
        Dispatch::Training(result)
    } else if command.command_type == COMMAND_TYPE_BED_SLEEP_INTENT {
        match routing::bed::route(
            authority,
            session.actor,
            &session.admitted,
            command,
            &session.item_view,
        )
        .await
        {
            Some(dispatch) => dispatch,
            None => return Err(ServeEnd::ControlLost),
        }
    } else if command.command_type == COMMAND_TYPE_USE_INTENT {
        match routing::use_item::route(
            authority,
            session.actor,
            &mut session.admitted,
            command,
            &mut session.item_view,
            &mut session.container_views,
            &session.visibility,
        )
        .await
        {
            Some(dispatch) => dispatch,
            None => return Err(ServeEnd::Disconnected),
        }
    } else if command.command_type == COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT {
        match routing::spell::route(authority, session.actor, &session.admitted, command).await {
            Some(dispatch) => dispatch,
            None => return Err(ServeEnd::Disconnected),
        }
    } else if command.command_type == COMMAND_TYPE_CONTAINER_VIEW_INTENT {
        match routing::container_view::route(
            authority,
            session.actor,
            &mut session.admitted,
            command,
            &mut session.container_views,
            &mut session.item_view,
        )
        .await
        {
            Some(dispatch) => dispatch,
            None => return Err(ServeEnd::Disconnected),
        }
    } else if command.command_type == oteryn_protocol_oteryn::depot::COMMAND_TYPE_DEPOT_INTENT {
        match routing::depot::route(
            authority,
            session.actor,
            &session.admitted,
            command,
            &session.item_view,
        )
        .await
        {
            Some(dispatch) => dispatch,
            None => return Err(ServeEnd::Disconnected),
        }
    } else if command.command_type
        == oteryn_protocol_oteryn::timed_item::COMMAND_TYPE_TIMED_ITEM_INTENT
    {
        match routing::timed_item::route(
            authority,
            session.actor,
            &mut session.admitted,
            command,
            &mut session.item_view,
        )
        .await
        {
            Some(dispatch) => dispatch,
            None => return Err(ServeEnd::Disconnected),
        }
    } else if command.command_type == COMMAND_TYPE_ITEM_MOVE_INTENT {
        match routing::item_move::route(
            authority,
            session.actor,
            &mut session.admitted,
            command,
            &mut session.item_view,
            &mut session.container_views,
            session.selected_capabilities,
        )
        .await
        {
            Some(dispatch) => dispatch,
            None => return Err(ServeEnd::Disconnected),
        }
    } else if command.command_type == COMMAND_TYPE_ITEM_LOOK_INTENT {
        routing::item_look::route(
            authority,
            session.actor,
            &mut session.admitted,
            command,
            session.selected_capabilities,
            &session.visibility,
            &session.item_view,
        )
        .await
    } else {
        services(stream, authority, session, command, outcome).await?
    })
}

async fn services<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    outcome: &mut CommandOutcome,
) -> Result<Dispatch, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    Ok(
        if crate::gameplay_transport::house_move_out::owns(command.command_type) {
            match routing::house_move_out::route(authority, &session.admitted, command).await {
                Some(dispatch) => dispatch,
                None => return Err(ServeEnd::Disconnected),
            }
        } else if matches!(command.command_type, 72 | 73) {
            match routing::cyclopedia::route(authority, &session.admitted, command).await {
                Some(dispatch) => dispatch,
                None => return Err(ServeEnd::Disconnected),
            }
        } else if crate::gameplay_transport::guild_intent::owns(command.command_type) {
            match routing::guild::route(authority, &session.admitted, command).await {
                Some(dispatch) => dispatch,
                None => return Err(ServeEnd::Disconnected),
            }
        } else if matches!(
            command.command_type,
            oteryn_protocol_oteryn::staff::COMMAND_TYPE_STAFF_REQUEST
                | oteryn_protocol_oteryn::staff::COMMAND_TYPE_STAFF_RECEIPT
        ) {
            match routing::staff::route(authority, &session.admitted, command).await {
                Some(dispatch) => dispatch,
                None => return Err(ServeEnd::Disconnected),
            }
        } else if command.command_type == COMMAND_TYPE_QUEST_LOG_QUERY {
            match routing::quest_log::route(
                authority,
                session.actor,
                &mut session.admitted,
                command,
                &mut session.quest_log,
            )
            .await
            {
                Some(dispatch) => dispatch,
                None => return Err(ServeEnd::Disconnected),
            }
        } else if (48..=50).contains(&command.command_type) {
            routing::vip::route(authority, &session.admitted, command).await
        } else if matches!(
            command.command_type,
            oteryn_protocol_oteryn::party::COMMAND_TYPE_PARTY_INTENT
                | oteryn_protocol_oteryn::party::COMMAND_TYPE_PARTY_QUERY
        ) {
            routing::party::route(
                authority,
                &session.admitted,
                command,
                &mut session.party_view,
            )
            .await
        } else if command.command_type == COMMAND_TYPE_CHAT_INTENT {
            routing::chat::route(authority, session.actor, &session.admitted, command).await
        } else if command.command_type == COMMAND_TYPE_NPC_TALK_INTENT {
            match routing::npc::route(
                authority,
                session.actor,
                &session.admitted,
                command,
                &mut session.buffered,
                &mut outcome.npc_relocation,
            )
            .await
            {
                Some(dispatch) => dispatch,
                None => return Err(ServeEnd::Disconnected),
            }
        } else if command.command_type == COMMAND_TYPE_NPC_TRADE_INTENT {
            match routing::npc_trade::route(
                authority,
                session.actor,
                &mut session.admitted,
                command,
                &mut session.item_view,
            )
            .await
            {
                Some(dispatch) => dispatch,
                None => return Err(ServeEnd::Disconnected),
            }
        } else if command.command_type == COMMAND_TYPE_ATTACK_TARGET_INTENT {
            routing::combat_intent::route(authority, session.actor, &mut session.admitted, command)
                .await
        } else if command.command_type == COMMAND_TYPE_FIGHT_MODES_INTENT {
            routing::combat_intent::route_modes(
                authority,
                session.actor,
                &mut session.admitted,
                command,
            )
            .await
        } else if command.command_type == COMMAND_TYPE_LOGOUT_INTENT {
            logout(stream, authority, session, command, outcome).await?
        } else if command.command_type == COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY {
            routing::achievements::route(authority, &session.admitted, command).await
        } else if command.command_type
            == oteryn_protocol_oteryn::appearance::COMMAND_TYPE_APPEARANCE_CHANGE
        {
            routing::appearance::route(authority, session.actor, &session.admitted, command).await
        } else if matches!(
            command.command_type,
            oteryn_protocol_oteryn::house::COMMAND_TYPE_HOUSE_INTENT
                | oteryn_protocol_oteryn::house::COMMAND_TYPE_HOUSE_QUERY
        ) {
            routing::house::route(authority, session.actor, &session.admitted, command).await
        } else if matches!(
            command.command_type,
            oteryn_protocol_oteryn::daily_reward::COMMAND_TYPE_DAILY_REWARD_CLAIM
                | oteryn_protocol_oteryn::daily_reward::COMMAND_TYPE_DAILY_REWARD_QUERY
        ) {
            routing::daily_reward::route(authority, session.actor, &session.admitted, command).await
        } else if command.command_type
            == oteryn_protocol_oteryn::world_event::COMMAND_TYPE_WORLD_EVENT_QUERY
        {
            match routing::world_event::route(authority, &session.admitted, command).await {
                Some(dispatch) => dispatch,
                None => return Err(ServeEnd::Disconnected),
            }
        } else if command.command_type == COMMAND_TYPE_HIGHSCORES_QUERY_V1 {
            routing::highscores::route(authority, &session.admitted, command).await
        } else {
            Dispatch::Unregistered
        },
    )
}
