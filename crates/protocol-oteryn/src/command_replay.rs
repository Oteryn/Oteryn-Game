//! Replay an already terminal command without interpreting replacement command bytes.
use crate::{
    CommandStatus, FoundationProtocolError, MAX_COMMAND_RESULT_PAYLOAD_BYTES, MessageType,
    decode_command_result, decode_field_number, decode_wire_envelope, push_scalar, read_varint,
    server_frame, skip_field,
};

/// The largest retained result frame: the bounded typed payload and protobuf overhead.
pub const MAX_RETAINED_RESULT_BYTES: usize = MAX_COMMAND_RESULT_PAYLOAD_BYTES + 128;

/// Re-envelope the original terminal result for the current connection and sequence.
/// The error and typed payload stay authoritative even if the retry changes its input.
pub fn encode_duplicate_replay(
    generation: u64,
    sequence: u64,
    retained: &[u8],
) -> Result<Vec<u8>, FoundationProtocolError> {
    if retained.len() > MAX_RETAINED_RESULT_BYTES {
        return Err(FoundationProtocolError::PayloadLimitExceeded);
    }
    if generation == 0 || sequence == 0 {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    let envelope = decode_wire_envelope(retained)?;
    if envelope.message_type() != MessageType::CommandResult {
        return Err(FoundationProtocolError::UnknownMessageType);
    }
    decode_command_result(envelope.payload())?;
    let payload = envelope.payload();
    let mut body = Vec::with_capacity(payload.len());
    let mut cursor = 0;
    while cursor < payload.len() {
        let start = cursor;
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        skip_field(payload, &mut cursor, wire)?;
        if field == 2 {
            push_scalar(&mut body, 2, CommandStatus::DuplicateReplay as u64);
        } else {
            // Preserve all authoritative fields, including revisions and unknown fields.
            body.extend_from_slice(&payload[start..cursor]);
        }
    }
    Ok(server_frame(
        MessageType::CommandResult,
        generation,
        sequence,
        &body,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{encode_command_error_result, encode_command_result};

    #[test]
    fn command_replay_preserves_authoritative_fields() -> Result<(), FoundationProtocolError> {
        // Independent protobuf fixture: CommandId 3, ACCEPTED, domain 1 revision 9,
        // typed payload [7, 8], and unknown field 15 = 42. Only status/generation/sequence change.
        let original = [
            8, 8, 16, 1, 24, 2, 34, 16, 8, 3, 16, 1, 34, 4, 8, 1, 16, 9, 42, 2, 7, 8, 120, 42,
        ];
        let expected = [
            8, 8, 16, 4, 24, 10, 34, 16, 8, 3, 16, 3, 34, 4, 8, 1, 16, 9, 42, 2, 7, 8, 120, 42,
        ];
        assert_eq!(encode_duplicate_replay(4, 10, &original)?, expected);
        Ok(())
    }

    #[test]
    fn command_replay_preserves_results_and_enforces_bounds() -> Result<(), FoundationProtocolError>
    {
        for original in [
            encode_command_result(1, 2, 3, CommandStatus::Accepted, &[9, 8])?,
            encode_command_result(1, 2, 3, CommandStatus::Rejected, &[])?,
            encode_command_error_result(1, 2, 3, FoundationProtocolError::NotSupported)?,
            encode_command_result(
                1,
                2,
                3,
                CommandStatus::Accepted,
                &vec![7; MAX_COMMAND_RESULT_PAYLOAD_BYTES],
            )?,
        ] {
            let replay = encode_duplicate_replay(4, 10, &original)?;
            let envelope = decode_wire_envelope(&replay)?;
            assert_eq!(envelope.connection_generation(), 4);
            assert_eq!(envelope.server_sequence(), 10);
            let replayed = decode_command_result(envelope.payload())?;
            let original_envelope = decode_wire_envelope(&original)?;
            let prior = decode_command_result(original_envelope.payload())?;
            assert_eq!(replayed.command_id, prior.command_id);
            assert_eq!(replayed.status, CommandStatus::DuplicateReplay);
            assert_eq!(replayed.payload, prior.payload);
            assert_eq!(replayed.error_code, prior.error_code);
        }
        assert!(encode_duplicate_replay(1, 1, &[]).is_err());
        assert!(encode_duplicate_replay(0, 1, &[]).is_err());
        assert_eq!(
            encode_duplicate_replay(1, 1, &vec![0; MAX_RETAINED_RESULT_BYTES + 1]),
            Err(FoundationProtocolError::PayloadLimitExceeded),
        );
        Ok(())
    }
}
