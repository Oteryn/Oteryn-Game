#[test]
fn client_handshakes_require_v1_protocol_profile_and_schema_semantics() {
    let id = test_uuid_v7(1);
    for (message_type, payload, expected) in [
        (
            1,
            test_client_bootstrap_payload(2, 1, 1, &id, &[]),
            FoundationProtocolError::ProtocolMajorMismatch,
        ),
        (
            1,
            test_client_bootstrap_payload(1, 2, 1, &id, &[]),
            FoundationProtocolError::TransportProfileMismatch,
        ),
        (
            1,
            test_client_bootstrap_payload(1, 1, 0, &id, &[]),
            FoundationProtocolError::MalformedEnvelope,
        ),
        (
            3,
            test_client_resume_payload(2, 1, 1, &id, &[]),
            FoundationProtocolError::ProtocolMajorMismatch,
        ),
        (
            3,
            test_client_resume_payload(1, 2, 1, &id, &[]),
            FoundationProtocolError::TransportProfileMismatch,
        ),
        (
            3,
            test_client_resume_payload(1, 1, 0, &id, &[]),
            FoundationProtocolError::MalformedEnvelope,
        ),
    ] {
        assert_eq!(
            decode_wire_envelope(&test_envelope(message_type, &payload)),
            Err(expected)
        );
    }
}

#[test]
fn handshake_ingress_requires_exact_non_nil_uuidv7_identities() {
    let valid = test_uuid_v7(1);
    let nil = [0u8; 16];
    let short = [0u8; 15];
    let world = test_uuid_v7(2);
    let channel = test_uuid_v7(3);
    for (message_type, payload) in [
        (1, test_client_bootstrap_payload(1, 1, 1, &short, &[])),
        (3, test_client_resume_payload(1, 1, 1, &nil, &[])),
        (
            2,
            test_server_accepted_payload(1, 1, 1, [&nil, &world, &channel], &[]),
        ),
        (4, test_server_resume_accepted_payload(2, 1, 1, &short, &[])),
    ] {
        assert_eq!(
            decode_wire_envelope(&test_envelope(message_type, &payload)),
            Err(FoundationProtocolError::InvalidWireIdentifier)
        );
    }
    assert!(
        decode_wire_envelope(&test_envelope(
            2,
            &test_server_accepted_payload(1, 1, 1, [&valid, &world, &channel], &[])
        ))
        .is_ok()
    );
}

#[test]
fn accepted_messages_require_nonzero_authority_counters_and_schema() {
    let session = test_uuid_v7(1);
    let world = test_uuid_v7(2);
    let channel = test_uuid_v7(3);
    for (message_type, payload) in [
        (
            2,
            test_server_accepted_payload(0, 1, 1, [&session, &world, &channel], &[]),
        ),
        (
            2,
            test_server_accepted_payload(1, 0, 1, [&session, &world, &channel], &[]),
        ),
        (
            2,
            test_server_accepted_payload(1, 1, 0, [&session, &world, &channel], &[]),
        ),
        (
            4,
            test_server_resume_accepted_payload(0, 1, 1, &session, &[]),
        ),
        (
            4,
            test_server_resume_accepted_payload(2, 0, 1, &session, &[]),
        ),
        (
            4,
            test_server_resume_accepted_payload(2, 1, 0, &session, &[]),
        ),
    ] {
        assert_eq!(
            decode_wire_envelope(&test_envelope(message_type, &payload)),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
    }
}

#[test]
fn semantic_handshakes_preserve_same_major_and_additive_unknown_field_compatibility()
-> Result<(), FoundationProtocolError> {
    let session = test_uuid_v7(1);
    let world = test_uuid_v7(2);
    let channel = test_uuid_v7(3);
    let mut payloads = [
        test_client_bootstrap_payload(1, 1, 2, &session, &[]),
        test_client_resume_payload(1, 1, 2, &session, &[]),
        test_server_accepted_payload(1, 1, 2, [&session, &world, &channel], &[]),
        test_server_resume_accepted_payload(2, 1, 2, &session, &[]),
    ];
    for (index, payload) in payloads.iter_mut().enumerate() {
        push_test_bytes_field(payload, 16, b"future-addition");
        let message_type = [1u8, 3, 2, 4][index];
        assert_eq!(
            decode_wire_envelope(&test_envelope(message_type, payload))?.payload(),
            payload
        );
    }
    Ok(())
}

#[test]
fn duplicate_singular_handshake_fields_fail_closed() {
    let session = test_uuid_v7(1);
    let world = test_uuid_v7(2);
    let channel = test_uuid_v7(3);
    let mut bootstrap = test_client_bootstrap_payload(1, 1, 1, &session, &[]);
    push_test_varint_field(&mut bootstrap, 1, 1);
    assert_eq!(
        decode_wire_envelope(&test_envelope(1, &bootstrap)),
        Err(FoundationProtocolError::MalformedEnvelope)
    );

    let mut accepted = test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], &[]);
    push_test_varint_field(&mut accepted, 4, 1);
    assert_eq!(
        decode_wire_envelope(&test_envelope(2, &accepted)),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
}

#[test]
fn protobuf_field_numbers_cannot_wrap_into_known_fields() -> Result<(), FoundationProtocolError> {
    let oversized_message_field = (1u64 << 32) + 1;
    let oversized_payload_field = (1u64 << 32) + 4;
    let mut raw = test_varint((oversized_message_field << 3) as usize);
    raw.push(0x01);
    raw.extend(test_varint(((oversized_payload_field << 3) | 2) as usize));
    raw.push(0x00);
    assert_eq!(
        decode_wire_envelope(&raw),
        Err(FoundationProtocolError::MalformedEnvelope)
    );

    let oversized_nested_field = (1u64 << 32) + 5;
    let mut payload = test_varint(((oversized_nested_field << 3) | 2) as usize);
    payload.push(0x00);
    assert_eq!(
        decode_wire_envelope(&test_envelope(1, &payload)),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    Ok(())
}

#[test]
fn bootstrap_metadata_limits_cover_build_id_and_capabilities() -> Result<(), FoundationProtocolError>
{
    let build_128 = vec![b'a'; 128];
    let id = test_uuid_v7(1);
    let payload = test_client_bootstrap_payload_with_material_and_build(
        1,
        1,
        1,
        &id,
        &[],
        &[0xaa],
        &build_128,
    );
    assert_eq!(
        decode_wire_envelope(&test_envelope(1, &payload))?.payload(),
        payload
    );

    let build_129 = vec![b'a'; 129];
    let mut payload = vec![0x3a];
    payload.extend(test_varint(build_129.len()));
    payload.extend_from_slice(&build_129);
    assert_eq!(
        decode_wire_envelope(&test_envelope(1, &payload)),
        Err(FoundationProtocolError::MalformedEnvelope)
    );

    let invalid_utf8 = [0x3a, 0x01, 0xff];
    assert_eq!(
        decode_wire_envelope(&test_envelope(1, &invalid_utf8)),
        Err(FoundationProtocolError::MalformedEnvelope)
    );

    let resume_build_129 = vec![b'a'; 129];
    let mut resume_payload = vec![0x42];
    resume_payload.extend(test_varint(resume_build_129.len()));
    resume_payload.extend_from_slice(&resume_build_129);
    assert_eq!(
        decode_wire_envelope(&test_envelope(3, &resume_payload)),
        Err(FoundationProtocolError::MalformedEnvelope)
    );

    let mut packed = Vec::new();
    for capability in 1..=128usize {
        packed.extend(test_varint(capability));
    }
    let mut payload = test_client_bootstrap_payload(1, 1, 1, &id, &[]);
    payload.push(0x22);
    payload.extend(test_varint(packed.len()));
    payload.extend_from_slice(&packed);
    assert!(decode_wire_envelope(&test_envelope(1, &payload)).is_ok());

    packed.extend(test_varint(129));
    let mut payload = vec![0x22];
    payload.extend(test_varint(packed.len()));
    payload.extend_from_slice(&packed);
    assert_eq!(
        decode_wire_envelope(&test_envelope(1, &payload)),
        Err(FoundationProtocolError::BootstrapLimitExceeded)
    );
    let mut resume_payload = vec![0x3a];
    resume_payload.extend(test_varint(packed.len()));
    resume_payload.extend_from_slice(&packed);
    assert_eq!(
        decode_wire_envelope(&test_envelope(3, &resume_payload)),
        Err(FoundationProtocolError::BootstrapLimitExceeded)
    );

    for invalid_caps in [[1usize, 1usize], [2usize, 1usize]] {
        let mut packed = Vec::new();
        for capability in invalid_caps {
            packed.extend(test_varint(capability));
        }
        let mut payload = vec![0x22];
        payload.extend(test_varint(packed.len()));
        payload.extend_from_slice(&packed);
        assert_eq!(
            decode_wire_envelope(&test_envelope(1, &payload)),
            Err(FoundationProtocolError::InvalidCapabilitySet)
        );
    }
    Ok(())
}
