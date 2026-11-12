//! Client codecs for the existing FND-02 recovery handshake. No new wire fields.
use super::*;

#[derive(Debug, Clone, Copy)]
pub struct ClientResumeValue<'a> {
    pub game_session_id: GameSessionId,
    pub reconnect_material: &'a [u8],
    pub last_applied_server_sequence: u64,
    pub schema_revision: u32,
    pub client_build_id: &'a str,
    pub supported_capabilities: &'a [u32],
}

pub fn encode_client_resume(
    value: &ClientResumeValue<'_>,
) -> Result<Vec<u8>, FoundationProtocolError> {
    if value.reconnect_material.len() > MAX_RECONNECT_MATERIAL_BYTES {
        return Err(FoundationProtocolError::BootstrapLimitExceeded);
    }
    if value.reconnect_material.is_empty()
        || value.schema_revision == 0
        || value.client_build_id.is_empty()
        || value.client_build_id.len() > MAX_CLIENT_BUILD_ID_BYTES
    {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    let mut count = 0;
    let mut previous = None;
    for &capability in value.supported_capabilities {
        validate_capability(u64::from(capability), &mut count, &mut previous, false)?;
    }
    let mut payload = Vec::new();
    push_bytes(&mut payload, 1, value.game_session_id.as_bytes());
    push_bytes(&mut payload, 2, value.reconnect_material);
    push_scalar(&mut payload, 3, value.last_applied_server_sequence);
    push_scalar(&mut payload, 4, u64::from(PROTOCOL_MAJOR_V1));
    push_scalar(&mut payload, 5, u64::from(TRANSPORT_PROFILE_TCP_TLS13_V1));
    push_scalar(&mut payload, 6, u64::from(value.schema_revision));
    for &capability in value.supported_capabilities {
        push_scalar(&mut payload, 7, u64::from(capability));
    }
    push_bytes(&mut payload, 8, value.client_build_id.as_bytes());
    let mut output = vec![8, MessageType::ClientResume as u8];
    push_bytes(&mut output, 4, &payload);
    // Apply the same limits, required fields and capability ordering as server ingress.
    decode_wire_envelope(&output)?.client_resume()?;
    Ok(output)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerResumeAcceptedFields {
    pub game_session_id: GameSessionId,
    pub connection_generation: u64,
    pub current_server_sequence: u64,
    pub next_command_id: u64,
    pub schema_revision: u32,
    pub selected_capabilities: Vec<u32>,
    pub connection_trace: Option<[u8; 16]>,
}

pub fn decode_server_resume_accepted(
    payload: &[u8],
) -> Result<ServerResumeAcceptedFields, FoundationProtocolError> {
    validate_server_acceptance_ingress(MessageType::ServerResumeAccepted, payload)?;
    let mut cursor = 0;
    let mut session = None;
    let (mut generation, mut sequence, mut next, mut schema) = (0, 0, 0, 0);
    let mut capabilities = Vec::new();
    let mut connection_trace = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match field {
            1 => {
                session = Some(GameSessionId::decode(bounded_length_delimited(
                    payload,
                    &mut cursor,
                    16,
                    FoundationProtocolError::InvalidWireIdentifier,
                )?)?)
            }
            2 => generation = read_varint(payload, &mut cursor)?,
            3 => sequence = read_varint(payload, &mut cursor)?,
            4 => next = read_varint(payload, &mut cursor)?,
            5 => schema = read_varint(payload, &mut cursor)?,
            6 if wire == 0 => capabilities.push(
                u32::try_from(read_varint(payload, &mut cursor)?)
                    .map_err(|_| FoundationProtocolError::InvalidCapabilitySet)?,
            ),
            6 => {
                let packed = bounded_length_delimited(
                    payload,
                    &mut cursor,
                    payload.len(),
                    FoundationProtocolError::MalformedEnvelope,
                )?;
                let mut packed_cursor = 0;
                while packed_cursor < packed.len() {
                    capabilities.push(
                        u32::try_from(read_varint(packed, &mut packed_cursor)?)
                            .map_err(|_| FoundationProtocolError::InvalidCapabilitySet)?,
                    );
                }
            }
            7 => read_connection_trace(payload, &mut cursor, wire, &mut connection_trace)?,
            _ => skip_field(payload, &mut cursor, wire)?,
        }
    }
    if connection_trace.is_some()
        && !capabilities.contains(&connection_trace::CAPABILITY_CONNECTION_TRACE_V1)
    {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    Ok(ServerResumeAcceptedFields {
        game_session_id: session.ok_or(FoundationProtocolError::InvalidWireIdentifier)?,
        connection_generation: generation,
        current_server_sequence: sequence,
        next_command_id: next,
        schema_revision: u32::try_from(schema)
            .map_err(|_| FoundationProtocolError::MalformedEnvelope)?,
        selected_capabilities: capabilities,
        connection_trace,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_codecs_match_existing_server_ingress_and_encoder()
    -> Result<(), FoundationProtocolError> {
        let id =
            GameSessionId::decode(&[1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16])?;
        let value = ClientResumeValue {
            game_session_id: id,
            reconnect_material: b"recovery",
            last_applied_server_sequence: 42,
            schema_revision: 1,
            client_build_id: "qualification",
            supported_capabilities: &[4, 6, 21],
        };
        let encoded = encode_client_resume(&value)?;
        let envelope = decode_wire_envelope(&encoded)?;
        let decoded = envelope.client_resume()?;
        assert_eq!(decoded.game_session_id, id);
        assert_eq!(decoded.last_applied_server_sequence, 42);
        assert_eq!(decoded.supported_capabilities, &[4, 6, 21]);
        let reply = encode_server_resume_accepted(&ServerResumeAcceptedValue {
            game_session_id: id,
            connection_generation: 2,
            current_server_sequence: 44,
            next_command_id: 8,
            schema_revision: 1,
            selected_capabilities: &[4, 6, 21],
        })?;
        let envelope = decode_wire_envelope(&reply)?;
        let accepted = decode_server_resume_accepted(envelope.payload())?;
        assert_eq!(accepted.game_session_id, id);
        assert_eq!(accepted.connection_generation, 2);
        assert_eq!(accepted.current_server_sequence, 44);
        assert_eq!(accepted.next_command_id, 8);
        assert_eq!(accepted.selected_capabilities, vec![4, 6, 21]);
        assert!(
            encode_client_resume(&ClientResumeValue {
                reconnect_material: b"",
                ..value
            })
            .is_err()
        );
        assert!(
            encode_client_resume(&ClientResumeValue {
                supported_capabilities: &[21, 4],
                ..value
            })
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn resume_acceptance_decodes_capability_gated_trace() -> Result<(), FoundationProtocolError> {
        let id =
            GameSessionId::decode(&[1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16])?;
        let trace = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
        let value = ServerResumeAcceptedValue {
            game_session_id: id,
            connection_generation: 2,
            current_server_sequence: 44,
            next_command_id: 8,
            schema_revision: 1,
            selected_capabilities: &[connection_trace::CAPABILITY_CONNECTION_TRACE_V1],
        };
        let frame = encode_server_resume_accepted_with_trace(&value, &trace)?;
        let envelope = decode_wire_envelope(&frame)?;
        assert_eq!(
            decode_server_resume_accepted(envelope.payload())?.connection_trace,
            Some(trace)
        );

        let mut unselected = envelope.payload().to_vec();
        let capability = [0x30, connection_trace::CAPABILITY_CONNECTION_TRACE_V1 as u8];
        let offset = unselected
            .windows(capability.len())
            .position(|window| window == capability)
            .ok_or(FoundationProtocolError::MalformedEnvelope)?;
        unselected.drain(offset..offset + capability.len());
        assert_eq!(
            decode_server_resume_accepted(&unselected),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        Ok(())
    }
}
