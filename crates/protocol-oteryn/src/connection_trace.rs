//! ERR-TRACE-5 diagnostic connection correlation.

pub const CAPABILITY_CONNECTION_TRACE_V1: u32 = 20;

/// A connection trace is a non-nil RFC 9562 UUIDv7 with the RFC variant.
pub(crate) fn valid(trace: &[u8]) -> bool {
    trace.len() == 16
        && trace.iter().any(|byte| *byte != 0)
        && trace[6] >> 4 == 7
        && trace[8] >> 6 == 2
}

#[cfg(test)]
mod tests {
    use crate::{
        FoundationProtocolError, MessageType, ProtocolDisposition, ServerAcceptedValue,
        connection_trace::CAPABILITY_CONNECTION_TRACE_V1, decode_protocol_error,
        decode_server_accepted, decode_wire_envelope, encode_protocol_error_with_trace,
        encode_server_accepted_with_trace,
    };

    const TRACE: [u8; 16] = [
        0x01, 0x9a, 0x6a, 0x01, 0x02, 0x03, 0x70, 0x05, 0x80, 0x07, 8, 9, 10, 11, 12, 13,
    ];

    fn accepted<'a>(capabilities: &'a [u32]) -> ServerAcceptedValue<'a> {
        use crate::{ChannelId, GameSessionId, WorldId};
        let mut id = TRACE;
        id[15] = 1;
        ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&id).unwrap(),
            world_id: WorldId::decode(&id).unwrap(),
            channel_id: ChannelId::decode(&id).unwrap(),
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: capabilities,
        }
    }

    #[test]
    fn acceptance_trace_round_trips_only_with_capability() {
        let frame =
            encode_server_accepted_with_trace(&accepted(&[CAPABILITY_CONNECTION_TRACE_V1]), &TRACE)
                .unwrap();
        let envelope = decode_wire_envelope(&frame).unwrap();
        assert_eq!(envelope.message_type(), MessageType::ServerAccepted);
        assert_eq!(
            decode_server_accepted(envelope.payload())
                .unwrap()
                .connection_trace,
            Some(TRACE)
        );
        assert_eq!(
            encode_server_accepted_with_trace(&accepted(&[]), &TRACE),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
    }

    #[test]
    fn protocol_error_trace_round_trips_and_rejects_invalid_uuid() {
        let frame =
            encode_protocol_error_with_trace(FoundationProtocolError::MalformedFrame, 1, &TRACE)
                .unwrap();
        let envelope = decode_wire_envelope(&frame).unwrap();
        let decoded = decode_protocol_error(envelope.payload()).unwrap();
        assert_eq!(decoded.error_code, 1001);
        assert_eq!(decoded.disposition, ProtocolDisposition::TransportFatal);
        assert_eq!(decoded.connection_trace, Some(TRACE));

        let mut invalid = TRACE;
        invalid[6] = 0x60;
        assert_eq!(
            encode_protocol_error_with_trace(FoundationProtocolError::MalformedFrame, 1, &invalid),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
    }
}
