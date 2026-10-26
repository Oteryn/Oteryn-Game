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
        let snapshot = encode_single_chunk_snapshot(
            admitted.continuity.connection_generation,
            sequence,
            sequence,
            &[DomainSnapshot {
                domain_id: STATE_DOMAIN_HOUSE_SCOPE,
                revision: 1,
                snapshot_type: 1,
                payload: &projection,
            }],
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
        let mut frames = FrameReader::default();
        let mut liveness = Liveness::new(policy);
        let mut cadence = tokio::time::interval_at(
            tokio::time::Instant::now() + policy.interval,
            policy.interval,
        );
        cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            let frame = {
                let mut read = std::pin::pin!(frames.next(stream));
                let mut tick = std::pin::pin!(cadence.tick());
                std::future::poll_fn(|cx| {
                    if tick.as_mut().poll(cx).is_ready() {
                        return std::task::Poll::Ready(None);
                    }
                    read.as_mut().poll(cx).map(Some)
                })
                .await
            };
            let Some(frame) = frame else {
                let probe = liveness.tick().map_err(|_| ServeEnd::ControlLost)?;
                let frame =
                    encode_liveness_probe(generation, probe).map_err(|_| ServeEnd::Disconnected)?;
                write_frame(stream, &frame)
                    .await
                    .map_err(|_| ServeEnd::Disconnected)?;
                continue;
            };
            let frame = frame.map_err(|_| ServeEnd::Disconnected)?;
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
            let permit = authority.enter_input(admitted.game_session_id).await;
            if permit.is_some() && !authority.house_transfer_input(admitted).await {
                return Err(ServeEnd::Disconnected);
            }
            match commands
                .begin(
                    command.command_id,
                    generation,
                    admitted.continuity.server_sequence,
                    false,
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
            let (next, payload) = if permit.is_some()
                && command.command_type == COMMAND_TYPE_HOUSE_TRANSFER
                && admitted
                    .continuity
                    .selected_capabilities
                    .command_selected(command.command_type)
            {
                authority
                    .house_transfer(admitted, command.command_id, command.payload)
                    .await
                    .ok_or(ServeEnd::Disconnected)?
            } else {
                (None, Vec::new())
            };
            drop(permit);
            let sequence = admitted
                .continuity
                .server_sequence
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let result = encode_command_result(
                generation,
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
            commands
                .terminalize_with(command.command_id, &result, || {})
                .ok_or(ServeEnd::Disconnected)?;
            admitted.continuity.server_sequence = sequence;
            admitted.continuity.next_command_id = commands.next_command_id();
            if let Some(next) = next {
                admitted = next;
            }
            write_frame(stream, &result)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
            if next.is_some() {
                return Ok(admitted);
            }
        }
    }
    .await;
    run.map_err(|end| Box::new(end.finish(admitted)))
}
