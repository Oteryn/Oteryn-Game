#[test]
fn envelope_oracle_bytes_decode_and_validate_direction() -> Result<(), FoundationProtocolError> {
    let bytes = [0x08, 0x07, 0x10, 0x01, 0x22, 0x04, 0x22, 0x02, 0xaa, 0xbb];
    let envelope = decode_wire_envelope(&bytes)?;
    assert_eq!(envelope.message_type(), MessageType::ClientCommand);
    assert_eq!(envelope.connection_generation(), 1);
    assert_eq!(envelope.server_sequence(), 0);
    assert_eq!(envelope.payload(), &[0x22, 0x02, 0xaa, 0xbb]);
    assert_eq!(envelope.validate(Direction::ClientToServer, true), Ok(()));
    assert_eq!(
        envelope.validate(Direction::ServerToClient, true),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    Ok(())
}

#[test]
fn additive_unknown_envelope_field_is_safely_ignored() -> Result<(), FoundationProtocolError> {
    let payload = test_client_bootstrap_payload(1, 1, 1, &test_uuid_v7(1), &[]);
    let mut bytes = test_envelope(1, &payload);
    bytes.extend([0x80, 0x01, 0x01]);
    let envelope = decode_wire_envelope(&bytes)?;
    assert_eq!(envelope.message_type(), MessageType::ClientBootstrap);
    assert_eq!(envelope.payload(), payload);
    Ok(())
}

#[test]
fn bootstrap_phase_requires_pre_admission_zero_generation() -> Result<(), FoundationProtocolError> {
    let payload = test_client_bootstrap_payload(1, 1, 1, &test_uuid_v7(1), &[]);
    let mut with_generation = vec![0x08, 0x01, 0x10, 0x01, 0x22];
    with_generation.extend(test_varint(payload.len()));
    with_generation.extend_from_slice(&payload);
    let envelope = decode_wire_envelope(&with_generation)?;
    assert_eq!(
        envelope.validate(Direction::ClientToServer, false),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    let zero_generation = test_envelope(1, &payload);
    let envelope = decode_wire_envelope(&zero_generation)?;
    assert_eq!(
        envelope.validate(Direction::ClientToServer, true),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    Ok(())
}

#[test]
fn unknown_and_truncated_envelopes_fail_closed() {
    assert_eq!(
        decode_wire_envelope(&[0x08, 0x63]),
        Err(FoundationProtocolError::UnknownMessageType)
    );
    assert_eq!(
        decode_wire_envelope(&[0x08]),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
}

#[test]
fn framed_envelope_rejects_truncation_and_oversized_prefix_before_body_access()
-> Result<(), FoundationProtocolError> {
    let valid = [0, 0, 0, 4, 0x08, 0x0e, 0x22, 0x00];
    assert_eq!(
        decode_framed_envelope(&valid)?.message_type(),
        MessageType::ProtocolError
    );
    assert_eq!(
        decode_framed_envelope(&[0, 0, 0, 5, 0x08, 0x0e, 0x22, 0x00]),
        Err(FoundationProtocolError::MalformedFrame)
    );
    assert_eq!(
        decode_framed_envelope(&[0, 0x10, 0, 1]),
        Err(FoundationProtocolError::FrameTooLarge)
    );
    Ok(())
}

#[test]
fn wire_identifier_requires_exact_nonzero_sixteen_bytes() -> Result<(), FoundationProtocolError> {
    assert_eq!(
        GameSessionId::decode(&[0; 15]),
        Err(FoundationProtocolError::InvalidWireIdentifier)
    );
    assert_eq!(
        GameSessionId::decode(&[0; 16]),
        Err(FoundationProtocolError::InvalidWireIdentifier)
    );
    let mut raw = [0u8; 16];
    raw[6] = 0x70;
    raw[8] = 0x80;
    raw[15] = 1;
    assert_eq!(GameSessionId::decode(&raw)?.as_bytes(), &raw);
    Ok(())
}
