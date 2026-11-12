//! Framing and exchange validation shared by session features.
use super::*;

/// A validated, correlated `CommandResult` with its typed payload still undecoded.
pub(super) struct ReceivedResult {
    pub(super) command_id: u64,
    pub(super) status: CommandStatus,
    pub(super) server_sequence: u64,
    pub(super) payload: Vec<u8>,
}

/// Checks a decoded `StateDelta` carries the registered `delta_type` and is based on exactly the
/// stored revision.
pub(super) fn check_delta(
    delta: &oteryn_protocol_oteryn::StateDeltaView<'_>,
    delta_type: u32,
    applied_revision: u64,
) -> Result<(), SessionError> {
    if delta.delta_type != delta_type {
        return Err(SessionError::UnregisteredDeltaType {
            domain_id: delta.domain_id,
            delta_type: delta.delta_type,
        });
    }
    if delta.base_revision != applied_revision {
        return Err(SessionError::StateRevisionMismatch {
            domain_id: delta.domain_id,
            expected_base: applied_revision,
            actual_base: delta.base_revision,
        });
    }
    Ok(())
}

/// A validated `StateDelta` with its domain-typed payload still undecoded.
pub(super) struct RawDelta {
    pub(super) server_sequence: u64,
    pub(super) base_revision: u64,
    pub(super) new_revision: u64,
    pub(super) payload: Vec<u8>,
}

/// Reads one bounded frame expected to be `expected` (`SnapshotBegin` or `SnapshotCommit`),
/// checks its envelope `connection_generation` matches the admitted session's, and returns its
/// `snapshot_id` (`decode_snapshot_id`) for the caller to correlate against `SnapshotBegin`'s.
pub(super) async fn read_snapshot_marker<S: SessionStream>(
    stream: &mut S,
    deadline: Duration,
    label: &'static str,
    expected: MessageType,
    session_generation: u64,
) -> Result<u64, SessionError> {
    let frame = bounded(deadline, label, read_frame(stream)).await?;
    let envelope = decode_wire_envelope(&frame)?;
    // Post-admission server traffic, same rules as `SnapshotBegin`/`SnapshotChunk`.
    envelope.validate(Direction::ServerToClient, true)?;
    if envelope.message_type() != expected {
        return Err(SessionError::UnexpectedMessage {
            expected,
            actual: envelope.message_type(),
        });
    }
    if envelope.connection_generation() != session_generation {
        return Err(SessionError::ConnectionGenerationMismatch {
            expected: session_generation,
            actual: envelope.connection_generation(),
        });
    }
    Ok(decode_snapshot_id(envelope.payload())?)
}

/// Bounds `future` by `deadline`, mapping an elapsed deadline to `SessionError::Timeout(label)`
/// and any inner error through `SessionError`'s existing `From` conversions.
pub(super) async fn bounded<T, E, F>(
    deadline: Duration,
    label: &'static str,
    future: F,
) -> Result<T, SessionError>
where
    F: Future<Output = Result<T, E>>,
    SessionError: From<E>,
{
    match tokio::time::timeout(deadline, future).await {
        Ok(result) => result.map_err(SessionError::from),
        Err(_elapsed) => Err(SessionError::Timeout(label)),
    }
}

/// Writes one length-prefixed frame. Generic over the stream type (client or server side of any
/// transport) so a fake peer can frame with the exact same code the session uses.
pub async fn write_frame<S: AsyncWrite + Unpin>(
    stream: &mut S,
    body: &[u8],
) -> Result<(), SessionError> {
    let length = FrameLength::new(
        u32::try_from(body.len()).map_err(|_error| FoundationProtocolError::FrameTooLarge)?,
    )?;
    stream.write_all(&length.to_prefix()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

/// Reads one length-prefixed frame body (bounded by `FrameLength`).
pub async fn read_frame<S: AsyncRead + Unpin>(stream: &mut S) -> Result<Vec<u8>, SessionError> {
    let mut prefix = [0_u8; 4];
    stream.read_exact(&mut prefix).await?;
    read_frame_body(stream, prefix).await
}

/// `read_frame` for a frame whose first length-prefix byte was already read.
pub(super) async fn read_frame_after<S: AsyncRead + Unpin>(
    stream: &mut S,
    first: u8,
) -> Result<Vec<u8>, SessionError> {
    let mut prefix = [first, 0, 0, 0];
    stream.read_exact(&mut prefix[1..]).await?;
    read_frame_body(stream, prefix).await
}

pub(super) async fn read_frame_body<S: AsyncRead + Unpin>(
    stream: &mut S,
    prefix: [u8; 4],
) -> Result<Vec<u8>, SessionError> {
    let length = FrameLength::from_prefix(&prefix)?;
    let mut body = vec![0_u8; length.get() as usize];
    stream.read_exact(&mut body).await?;
    Ok(body)
}

/// The server sends `ACCEPTED` for every typed disposition except `REJECTED`, which it sends
/// with `REJECTED` (`connection.rs` `serve_admitted`, both `STEP` and `USE_INTENT`); any other
/// pairing is a violation.
/// A server that sends a capability 11 disposition (11 to 17) or a gate message id to a session
/// that did not select capability 11 broke the negotiation: the result fails closed. So does a
/// capability 22 disposition (18 or 19) to a session that did not select capability 22.
pub(super) fn check_use_result_negotiated(
    disposition: UseDisposition,
    gate_message_id: Option<u8>,
    world_interaction_selected: bool,
    rune_use_selected: bool,
) -> Result<(), SessionError> {
    if ((disposition.needs_world_interaction() || gate_message_id.is_some())
        && !world_interaction_selected)
        || (disposition.needs_rune_use() && !rune_use_selected)
    {
        return Err(SessionError::WorldObject(
            world_object::WorldObjectError::Malformed,
        ));
    }
    Ok(())
}

pub(super) fn check_status_pairing(
    command_id: u64,
    status: CommandStatus,
    disposition_rejected: bool,
) -> Result<(), SessionError> {
    let consistent = match status {
        CommandStatus::Accepted => !disposition_rejected,
        CommandStatus::Rejected => disposition_rejected,
        CommandStatus::DuplicateReplay | CommandStatus::DuplicateOutcomeExpired => false,
    };
    if consistent {
        Ok(())
    } else {
        Err(SessionError::InconsistentCommandResult { command_id, status })
    }
}
