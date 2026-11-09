//! Every activity participant crosses the source stream's scope barrier before
//! any destination snapshot. Non-initiators have no fabricated command result.
use super::*;
use oteryn_protocol_oteryn::activity_scope::STATE_DOMAIN_ACTIVITY_SCOPE;

pub(super) async fn pending<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<Option<(AdmittedSession, Option<u64>)>, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let Some(switch) = authority
        .pending_activity_switch(session.admitted)
        .await
        .map_err(|_| ServeEnd::Disconnected)?
    else {
        return Ok(None);
    };
    let source = session.admitted;
    let receipt = oteryn_protocol_oteryn::activity_scope::decode_activity_scope(&switch.payload)
        .map_err(|_| ServeEnd::Disconnected)?;
    session.activity_ack =
        receipt
            .requires_ack
            .then_some(oteryn_protocol_oteryn::activity_scope::ActivityScopeAck {
                handoff: receipt.handoff,
                session: receipt.session,
            });
    authority.cast_end(source).await;
    let sequence = session
        .sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let frames = encode_single_chunk_snapshot(
        session.generation,
        sequence,
        sequence,
        &[DomainSnapshot {
            domain_id: STATE_DOMAIN_ACTIVITY_SCOPE,
            revision: 1,
            snapshot_type: 1,
            payload: &switch.payload,
        }],
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    // SQL and native installation already changed the current session. A socket
    // failure while sending the barrier must lose the exact destination control,
    // rather than try to release the terminal source or reset a stale generation.
    session.admitted = switch.destination;
    for frame in frames {
        write_frame(stream, &frame)
            .await
            .map_err(|_| ServeEnd::Disconnected)?;
    }
    if !authority.finish_activity_switch(source, switch.destination) {
        return Err(ServeEnd::Disconnected);
    }
    Ok(Some((switch.destination, switch.movement_effect_start)))
}

/// A source command may already be in TCP, even when both fresh sessions have
/// connection generation one. Keep the incremental reader (including a partial
/// source frame) and discard source input until the client confirms this exact
/// complete destination. No discarded source input reaches gameplay dispatch.
pub(super) async fn acknowledge<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    expected: oteryn_protocol_oteryn::activity_scope::ActivityScopeAck,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let timeout = session
        .policy
        .interval
        .saturating_mul(session.policy.missed_limit);
    let frame = tokio::time::timeout(
        timeout,
        read_acknowledgement(stream, &mut session.frames, session.generation, expected),
    )
    .await
    .map_err(|_| ServeEnd::ControlLost("activity_transfer"))??;
    let envelope = decode_wire_envelope(&frame).map_err(|_| ServeEnd::Disconnected)?;
    let command = envelope
        .client_command(session.generation)
        .map_err(|_| ServeEnd::Disconnected)?;
    use crate::gameplay_transport::session_ingress::CommandAction;
    match session.commands.begin(
        command.command_id,
        session.generation,
        session.sequence,
        false,
    ) {
        Ok(CommandAction::Execute) => {}
        _ => return Err(ServeEnd::Disconnected),
    }
    let sequence = session
        .sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let result = encode_command_result(
        session.generation,
        sequence,
        command.command_id,
        CommandStatus::Accepted,
        &[],
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    session
        .commands
        .terminalize_with(command.command_id, &result, || {})
        .ok_or(ServeEnd::Disconnected)?;
    session.sequence = sequence;
    session.admitted.continuity.server_sequence = sequence;
    session.admitted.continuity.next_command_id = session.commands.next_command_id();
    if !authority.remember_activity_connection(session.admitted) {
        return Err(ServeEnd::Disconnected);
    }
    write_frame(stream, &result)
        .await
        .map_err(|_| ServeEnd::Disconnected)
}

async fn read_acknowledgement<S: AsyncRead + Unpin>(
    stream: &mut S,
    frames: &mut FrameReader,
    generation: u64,
    expected: oteryn_protocol_oteryn::activity_scope::ActivityScopeAck,
) -> Result<Vec<u8>, ServeEnd> {
    use oteryn_protocol_oteryn::activity_scope::{
        COMMAND_TYPE_ACTIVITY_SCOPE_ACK, decode_activity_scope_ack,
    };
    for _ in 0..=MAX_FRAMES_HELD_BEHIND_A_BUFFERED_STEP {
        let frame = frames
            .next(stream)
            .await
            .map_err(|_| ServeEnd::Disconnected)?;
        let envelope = decode_wire_envelope(&frame).map_err(|_| ServeEnd::Disconnected)?;
        envelope
            .validate(crate::foundation::Direction::ClientToServer, true)
            .map_err(|_| ServeEnd::Disconnected)?;
        if envelope.message_type() == MessageType::ClientCommand {
            let command = envelope
                .client_command(envelope.connection_generation())
                .map_err(|_| ServeEnd::Disconnected)?;
            if envelope.connection_generation() == generation
                && command.command_id == 1
                && command.command_type == COMMAND_TYPE_ACTIVITY_SCOPE_ACK
                && decode_activity_scope_ack(command.payload).is_ok_and(|ack| ack == expected)
            {
                return Ok(frame);
            }
        }
    }
    Err(ServeEnd::Disconnected)
}

#[cfg(test)]
mod tests;
