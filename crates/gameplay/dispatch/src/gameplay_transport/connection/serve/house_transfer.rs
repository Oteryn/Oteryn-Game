//! The scope barrier belongs to the connection; the owner decides the transfer.
use super::*;
use oteryn_protocol_oteryn::house_transfer::{
    COMMAND_TYPE_HOUSE_TRANSFER, STATE_DOMAIN_HOUSE_SCOPE,
};

pub(super) async fn execute<S: AsyncRead + AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    permit: Option<InputPermit<'_>>,
) -> Result<(), ServeEnd> {
    let Some((next, payload)) = authority
        .house_transfer(session.admitted, command.command_id, command.payload)
        .await
    else {
        return Err(ServeEnd::Disconnected);
    };
    drop(permit);
    let sequence = session
        .sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let result = encode_command_result(
        session.generation,
        sequence,
        command.command_id,
        if next.is_some() {
            CommandStatus::Accepted
        } else {
            CommandStatus::Rejected
        },
        &payload,
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    session
        .commands
        .terminalize_with(command.command_id, &result, || {})
        .ok_or(ServeEnd::Disconnected)?;
    session.admitted.continuity.server_sequence = sequence;
    session.admitted.continuity.next_command_id = session.commands.next_command_id();
    session.sequence = sequence;
    if let Some(next) = next {
        session.admitted = next;
    }
    write_frame(stream, &result)
        .await
        .map_err(|_| ServeEnd::Disconnected)?;
    match next {
        Some(next) => Err(ServeEnd::Transferred(Box::new(next))),
        None => Ok(()),
    }
}

/// House custody uses the same command ingress and framing, with no Channel
/// snapshot, combat cadence or Channel mutations while the House owns the slot.
pub(super) async fn serve<S: AsyncRead + AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    mut admitted: AdmittedSession,
    authority: &A,
    policy: LivenessPolicy,
    integrity: &mut integrity::Integrity,
) -> Result<AdmittedSession, Box<ConnectionEnd>> {
    let run = async {
        let (_, projection) = authority
            .house_scope_ack(admitted)
            .await
            .map_err(|_| ServeEnd::Disconnected)?
            .ok_or(ServeEnd::Disconnected)?;
        let sequence = admitted
            .continuity
            .server_sequence
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        let actor = admitted.runtime_actor.ok_or(ServeEnd::Disconnected)?;
        let selected = admitted.continuity.selected_capabilities;
        let (mut services, item_domains) =
            house_hireling::State::prepare(authority, actor, &mut admitted).await?;
        let mut domains = vec![DomainSnapshot {
            domain_id: STATE_DOMAIN_HOUSE_SCOPE,
            revision: 1,
            snapshot_type: 1,
            payload: &projection,
        }];
        domains.extend(item_domains.iter().map(|domain| DomainSnapshot {
            domain_id: domain.domain_id,
            revision: domain.revision,
            snapshot_type: domain.snapshot_type,
            payload: &domain.payload,
        }));
        services.appearance_domain(&mut domains);
        let snapshot = encode_single_chunk_snapshot(
            admitted.continuity.connection_generation,
            sequence,
            sequence,
            &domains,
        )
        .map_err(|_| ServeEnd::Disconnected)?;
        for frame in snapshot {
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
        }
        admitted.continuity.server_sequence = sequence;
        let mut commands = crate::gameplay_transport::session_ingress::SessionCommandStream::open(
            authority.session_commands(),
            &admitted,
        )
        .ok_or(ServeEnd::Disconnected)?;
        let generation = admitted.continuity.connection_generation;
        let mut queue = house_hireling::step::Queue::default();
        let mut liveness = Liveness::new(policy);
        let mut cadence = tokio::time::interval_at(
            tokio::time::Instant::now() + policy.interval,
            policy.interval,
        );
        cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut service_cadence = tokio::time::interval_at(
            tokio::time::Instant::now() + std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(1),
        );
        service_cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            let inbound::Input {
                frame,
                arrived_while_buffered,
                buffered_original,
            } = match queue
                .next(stream, generation, &mut cadence, &mut service_cadence)
                .await?
            {
                house_hireling::step::Event::Services => {
                    services.tick(stream, authority, &mut admitted).await?;
                    continue;
                }
                house_hireling::step::Event::Probe => {
                    let probe = liveness.tick().map_err(|_| ServeEnd::ControlLost)?;
                    let frame = encode_liveness_probe(generation, probe)
                        .map_err(|_| ServeEnd::Disconnected)?;
                    write_frame(stream, &frame)
                        .await
                        .map_err(|_| ServeEnd::Disconnected)?;
                    continue;
                }
                house_hireling::step::Event::Input(input) => input,
            };
            let envelope = decode_wire_envelope(&frame).map_err(ServeEnd::Closed)?;
            if envelope.connection_generation() != generation {
                return Err(ServeEnd::Closed(
                    FoundationProtocolError::StaleConnectionGeneration,
                ));
            }
            if envelope.message_type() == MessageType::LivenessAck {
                liveness
                    .ack(
                        envelope
                            .liveness_ack(generation)
                            .map_err(ServeEnd::Closed)?
                            .probe_id,
                    )
                    .map_err(ServeEnd::Closed)?;
                continue;
            }
            let command = envelope
                .client_command(generation)
                .map_err(ServeEnd::Closed)?;
            let limited = if buffered_original {
                false
            } else {
                integrity.admit().await
            };
            match commands
                .begin(
                    command.command_id,
                    generation,
                    admitted.continuity.server_sequence,
                    buffered_original,
                )
                .map_err(|(e, _)| ServeEnd::Closed(e))?
            {
                crate::gameplay_transport::session_ingress::CommandAction::Pending => continue,
                crate::gameplay_transport::session_ingress::CommandAction::Replay(frame) => {
                    admitted.continuity.server_sequence += 1;
                    write_frame(stream, &frame)
                        .await
                        .map_err(|_| ServeEnd::Disconnected)?;
                    continue;
                }
                crate::gameplay_transport::session_ingress::CommandAction::Execute => {}
            }
            let permit = if limited {
                None
            } else {
                authority.enter_input(admitted.game_session_id).await
            };
            if permit.is_some() && !authority.house_transfer_input(admitted).await {
                return Err(ServeEnd::Disconnected);
            }
            let mut effects = None;
            let mut second_logout = false;
            let mut lost_while_logging_out = false;
            let (next, payload, status) = if permit.is_some()
                && command.command_type == COMMAND_TYPE_HOUSE_TRANSFER
                && admitted
                    .continuity
                    .selected_capabilities
                    .command_selected(command.command_type)
            {
                let (next, payload) = authority
                    .house_transfer(admitted, command.command_id, command.payload)
                    .await
                    .ok_or(ServeEnd::Disconnected)?;
                let status = if next.is_some() {
                    CommandStatus::Accepted
                } else {
                    CommandStatus::Rejected
                };
                (next, payload, status)
            } else if permit.is_some()
                && command.command_type == oteryn_protocol_oteryn::house_wrap::COMMAND_WRAP
                && selected.command_selected(command.command_type)
            {
                match routing::house_wrap::route(authority, &admitted, command).await {
                    Some(Dispatch::Reply(status, payload)) => (None, payload, status),
                    _ => return Err(ServeEnd::Disconnected),
                }
            } else if permit.is_some() {
                let dispatch = if command.command_type == COMMAND_TYPE_LOGOUT_INTENT
                    && selected.command_selected(command.command_type)
                {
                    if decode_logout_intent(command.payload).is_err() {
                        Dispatch::Unregistered
                    } else {
                        let outcome = queue.logout(stream, authority, admitted).await?;
                        second_logout = outcome.second;
                        lost_while_logging_out = outcome.lost;
                        outcome.dispatch
                    }
                } else if command.command_type == COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT
                    && selected.command_selected(command.command_type)
                {
                    match services
                        .step(authority, &admitted, command, arrived_while_buffered)
                        .await
                    {
                        route::StepAction::Wait(due) => {
                            integrity.observe(&Dispatch::Step(StepOutcome::too_early(), 0));
                            // The reserved original runs once when due. Release the input
                            // permit while the bounded queue keeps later frames in order.
                            queue.buffer(frame, due);
                            continue;
                        }
                        route::StepAction::Dispatch(dispatch) => dispatch,
                    }
                } else {
                    services
                        .route(authority, &mut admitted, command)
                        .await
                        .ok_or(ServeEnd::Disconnected)?
                };
                integrity.observe(&dispatch);
                let (status, payload) = house_hireling::result(selected, &dispatch)?;
                effects = Some(dispatch);
                (None, payload, status)
            } else {
                (None, Vec::new(), CommandStatus::Rejected)
            };
            drop(permit);
            let sequence = admitted
                .continuity
                .server_sequence
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let result =
                encode_command_result(generation, sequence, command.command_id, status, &payload)
                    .map_err(|_| ServeEnd::Disconnected)?;
            commands
                .terminalize_with(command.command_id, &result, || {})
                .ok_or(ServeEnd::Disconnected)?;
            admitted.continuity.server_sequence = sequence;
            admitted.continuity.next_command_id = commands.next_command_id();
            if let Some(next) = next {
                admitted = next;
            }
            if matches!(effects, Some(Dispatch::Logout(LogoutResult::Accepted))) {
                // The native release already committed. A failed final write
                // remains a logout, never a second control-loss transition.
                if write_frame(stream, &result).await.is_ok()
                    && (!second_logout
                        || send_error(
                            stream,
                            FoundationProtocolError::TooManyOutstandingCommands,
                            generation,
                        )
                        .await
                        .is_ok())
                {
                    let _ = tokio::time::timeout(
                        policy.interval,
                        tokio::io::AsyncWriteExt::shutdown(stream),
                    )
                    .await;
                }
                return Err(ServeEnd::LoggedOut);
            }
            write_frame(stream, &result)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
            if second_logout {
                return Err(close(
                    stream,
                    admitted,
                    FoundationProtocolError::TooManyOutstandingCommands,
                )
                .await);
            }
            if lost_while_logging_out {
                return Err(ServeEnd::Disconnected);
            }
            if command.command_type == oteryn_protocol_oteryn::house_wrap::COMMAND_WRAP
                && matches!(
                    oteryn_protocol_oteryn::house_wrap::Reply::decode(&payload),
                    Some(oteryn_protocol_oteryn::house_wrap::Reply::Committed(_))
                )
            {
                services.inventory(stream, authority, &mut admitted).await?;
            }
            if let Some(effects) = effects {
                services
                    .effects(stream, authority, &mut admitted, effects)
                    .await?;
            }
            if next.is_some() {
                return Ok(admitted);
            }
        }
    }
    .await;
    run.map_err(|end| Box::new(end.finish(admitted)))
}
