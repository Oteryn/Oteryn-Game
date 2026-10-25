//! Decode, reserve and execute one command before producing its ordered output.

use super::*;

pub(super) struct CommandOutcome {
    pub(super) second_logout: bool,
    pub(super) lost_while_logging_out: bool,
    pub(super) npc_relocation: Option<WorldSpatialObservation>,
    pub(super) training_logged_out: bool,
}

pub(super) async fn execute<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    input: inbound::Input,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let inbound::Input {
        frame,
        arrived_while_buffered,
        buffered_original,
    } = input;
    let command = match decode_wire_envelope(&frame) {
        Err(error) => return Err(close(stream, session.admitted, error).await),
        Ok(envelope) if envelope.connection_generation() != session.generation => {
            return Err(close(
                stream,
                session.admitted,
                FoundationProtocolError::StaleConnectionGeneration,
            )
            .await);
        }
        Ok(envelope) if envelope.message_type() == MessageType::LivenessAck => {
            match envelope
                .liveness_ack(session.generation)
                .and_then(|ack| session.liveness.ack(ack.probe_id))
            {
                Ok(()) => return Ok(()),
                Err(error) => return Err(close(stream, session.admitted, error).await),
            }
        }
        Ok(envelope) if envelope.message_type() != MessageType::ClientCommand => {
            return Err(close(
                stream,
                session.admitted,
                FoundationProtocolError::UnknownMessageType,
            )
            .await);
        }
        Ok(envelope) => match envelope.client_command(session.generation) {
            Ok(command) => command,
            Err(error) => return Err(close(stream, session.admitted, error).await),
        },
    };
    match session.commands.begin(
        command.command_id,
        session.generation,
        session.sequence,
        buffered_original,
    ) {
        Err((error, expected)) => {
            let _ = match encode_command_protocol_error(
                error,
                session.generation,
                command.command_id,
                expected,
            ) {
                Ok(frame) => write_frame(stream, &frame).await,
                Err(_) => Ok(()),
            };
            return Err(ServeEnd::Closed(error));
        }
        Ok(crate::gameplay_transport::session_ingress::CommandAction::Replay(frame)) => {
            session.sequence += 1; // begin checked the successor before encoding the replay.
            session.admitted.continuity.server_sequence = session.sequence;
            if write_frame(stream, &frame).await.is_err() {
                return Err(ServeEnd::Disconnected);
            }
            return Ok(());
        }
        Ok(crate::gameplay_transport::session_ingress::CommandAction::Pending) => return Ok(()),
        Ok(crate::gameplay_transport::session_ingress::CommandAction::Execute) => {}
    }
    session.admitted.continuity.next_command_id = session.commands.next_command_id();
    // Unknown command types and malformed payloads have no effect. The result payload
    // belongs to the command type, so an unregistered type gets none. USE_INTENT (command
    // type 2, USE-WIRE-V1 #162 5868482467) is dispatched under the same FND-02 CommandId
    // session ingress above for every command. A reserved CommandId never becomes a
    // second operation, including across a same-GameSession connection replacement.
    // Spell retries likewise never cast or pay a second time (SPELL-D3).
    // ACCOUNT_ACHIEVEMENTS_QUERY (command type 10, display contract D223-D228) is a read of
    // the session's own account: the admitted controller's, never the payload's; a session
    // without a controller binding has none and is REJECTED.

    // LOGOUT-RL-01: a second logout read before the first one's result.
    let second_logout = false;
    // A read that failed while a logout ran: the peer is gone once the result is written.
    let lost_while_logging_out = false;
    // CAP-NEG-1: a command type owned by a capability the session did not select is refused
    // like an unregistered one, before any decode or authority call.
    // OPS-SAVE-2: the permit is held while the command runs, not while its result is
    // written, so the shutdown snapshot waits for the effect only.
    let mut input_permit = authority.enter_input().await;
    if input_permit.is_none() && session.reconcile_original_bank {
        // No current input grant: do not run money work, and do not falsely terminate an
        // accepted original command whose retained Bank status has not been classified.
        return Err(ServeEnd::Disconnected);
    }
    let retained_bank = if input_permit.is_some() && session.reconcile_original_bank {
        let outcome = authority
            .resume_npc_bank(
                session.actor,
                UseCommand {
                    game_session_id: session.admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: session.admitted.item_fence,
                },
            )
            .await;
        if outcome.as_ref().is_some_and(|outcome| outcome.unknown) {
            return Err(ServeEnd::Disconnected);
        }
        session.reconcile_original_bank = false;
        outcome
    } else {
        None
    };
    let npc_relocation = None;
    if input_permit.is_some()
        && !authority
            .training_activity(session.admitted, command.command_type)
            .await
    {
        return Err(ServeEnd::Disconnected);
    }
    let mut outcome = CommandOutcome {
        second_logout,
        lost_while_logging_out,
        npc_relocation,
        training_logged_out: false,
    };
    let mut before_use =
        if command.command_type == oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT {
            authority.observe(session.actor).await
        } else {
            None
        };
    if command.command_type != oteryn_protocol_oteryn::stash::COMMAND_TYPE_STASH_INTENT {
        session.stash_rows = None;
    }
    let walk_target = autowalk::target(session, command);
    let mut dispatch = if let Some(outcome) = retained_bank {
        Dispatch::Npc(
            outcome.disposition,
            NpcUpdate {
                conversation: outcome.conversation,
                window: outcome.window,
            },
            outcome.inventory_changed,
        )
    } else if input_permit.is_none()
        || !session
            .admitted
            .continuity
            .selected_capabilities
            .command_selected(command.command_type)
    {
        Dispatch::Unregistered
    } else if command.command_type == COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT {
        match route::step(authority, session, command, arrived_while_buffered).await {
            route::StepAction::Wait(due) => {
                session.buffered = Some(BufferedStep { frame, due });
                return Ok(());
            }
            route::StepAction::Dispatch(dispatch) => dispatch,
        }
    } else {
        route::execute(stream, authority, session, command, &mut outcome).await?
    };
    let mut no_path = false;
    if autowalk::too_far(&dispatch)
        && let Some(target) = walk_target
    {
        // Pacing waits must not hold shutdown's player-input permit. The walk takes one
        // permit per actual step, and the original action reacquires one before its retry.
        drop(input_permit.take());
        let walked = Box::pin(autowalk::walk(
            stream,
            authority,
            session,
            command.command_id,
            target,
        ))
        .await?;
        input_permit = authority.enter_input().await;
        match walked {
            autowalk::WalkOutcome::Arrived if input_permit.is_some() => {
                // The walk already published its steps. Only relocation by the retried USE
                // needs another spatial update after its result.
                if before_use.is_some() {
                    before_use = authority.observe(session.actor).await;
                }
                // Exactly one retry of the original CommandId and bytes. Source identity,
                // custody, reach, capacity and fences are read by the existing route.
                dispatch = Box::pin(route::execute(
                    stream,
                    authority,
                    session,
                    command,
                    &mut outcome,
                ))
                .await?;
            }
            autowalk::WalkOutcome::Arrived | autowalk::WalkOutcome::Cancelled => {}
            autowalk::WalkOutcome::NoPath => no_path = true,
        }
    }
    if before_use.is_some()
        && matches!(&dispatch, Dispatch::Use(outcome) if outcome.disposition == UseDisposition::Committed)
    {
        outcome.npc_relocation = newhaven_travel_transport::retire_buffer(
            authority,
            session.actor,
            before_use,
            &mut session.buffered,
        )
        .await;
    }
    dispatch = bank_inventory(authority, session, dispatch).await?;
    drop(input_permit);
    if matches!(dispatch, Dispatch::Appearance(None) | Dispatch::House(None)) {
        return Err(ServeEnd::Disconnected);
    }
    if matches!(dispatch, Dispatch::Vip(None)) {
        return Err(ServeEnd::Disconnected);
    }
    let Some(result_sequence) = session.sequence.checked_add(1) else {
        return Err(ServeEnd::Disconnected);
    };
    session.sequence = result_sequence;
    session.admitted.continuity.server_sequence = session.sequence;
    outcome.training_logged_out =
        matches!(&dispatch, Dispatch::Training(result) if result.disposition == 3);
    let result = result::encode(session, command, &mut dispatch)?;
    outbound::finish(
        stream, authority, session, command, dispatch, result, outcome,
    )
    .await?;
    if no_path {
        use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
        tutorial_notices_transport::emit_notice(
            stream,
            &mut session.admitted,
            &mut session.sequence,
            &TutorialNotice::StatusText("There is no way.".into()),
        )
        .await
        .ok_or(ServeEnd::Disconnected)?;
    }
    Ok(())
}

async fn bank_inventory<A>(
    authority: &A,
    session: &mut ServeState,
    dispatch: Dispatch,
) -> Result<Dispatch, ServeEnd>
where
    A: FreshAdmissionAuthority,
{
    Ok(
        // Native deposit/withdraw custody changes use the same inventory refresh as a trade.
        // Typed refusals and balance-only transfers do not manufacture an inventory mutation.
        match dispatch {
            Dispatch::Npc(disposition, update, true) => {
                let mut deltas = Vec::new();
                if let Some(view) = session.item_view.as_mut() {
                    let Some(inventory) = item_move::observe_inventory(
                        authority,
                        session.actor,
                        session.admitted.game_session_id,
                        view,
                    )
                    .await
                    else {
                        return Err(ServeEnd::Disconnected);
                    };
                    let Ok(delta) = view.inventory_committed(inventory) else {
                        return Err(ServeEnd::Disconnected);
                    };
                    session.admitted.continuity.item_view = view.continuity();
                    deltas.extend(delta);
                }
                Dispatch::NpcTrade(disposition, update, deltas)
            }
            other => other,
        },
    )
}
