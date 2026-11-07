#[test]
fn client_command_limits_cover_payload_and_expected_revisions()
-> Result<(), FoundationProtocolError> {
    let mut payload = vec![0x22];
    payload.extend(test_varint(65_536));
    payload.resize(payload.len() + 65_536, 0);
    assert!(decode_wire_envelope(&test_envelope(7, &payload)).is_ok());

    let mut payload = vec![0x22];
    payload.extend(test_varint(65_537));
    payload.resize(payload.len() + 65_537, 0);
    assert_eq!(
        decode_wire_envelope(&test_envelope(7, &payload)),
        Err(FoundationProtocolError::PayloadLimitExceeded)
    );

    let mut payload = Vec::new();
    for domain in 1..=64usize {
        let revision = test_state_revision(domain);
        payload.push(0x1a);
        payload.extend(test_varint(revision.len()));
        payload.extend(revision);
    }
    assert!(decode_wire_envelope(&test_envelope(7, &payload)).is_ok());

    let revision = test_state_revision(65);
    payload.push(0x1a);
    payload.extend(test_varint(revision.len()));
    payload.extend(revision);
    assert_eq!(
        decode_wire_envelope(&test_envelope(7, &payload)),
        Err(FoundationProtocolError::PayloadLimitExceeded)
    );

    let revision = test_state_revision(1);
    let mut duplicate = Vec::new();
    for _ in 0..2 {
        duplicate.push(0x1a);
        duplicate.extend(test_varint(revision.len()));
        duplicate.extend_from_slice(&revision);
    }
    assert_eq!(
        decode_wire_envelope(&test_envelope(7, &duplicate)),
        Err(FoundationProtocolError::StateRevisionMismatch)
    );
    Ok(())
}

#[test]
fn liveness_ack_requires_one_nonzero_probe_and_validates_optional_diagnostic() {
    for payload in [
        vec![0x08, 0x01],
        vec![0x08, 0x01, 0x10, 0x00],
        vec![0x10, 0x2a, 0x08, 0x01],
        vec![0x08, 0x01, 0x18, 0x07],       // additive unknown varint
        vec![0x08, 0x01, 0x22, 0x01, 0xff], // additive unknown bytes
        vec![
            0x08, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01,
        ],
    ] {
        assert!(decode_wire_envelope(&test_envelope(6, &payload)).is_ok());
    }

    for payload in [
        vec![],
        vec![0x10, 0x01],
        vec![0x08, 0x00],
        vec![0x08, 0x01, 0x08, 0x02],
        vec![0x08, 0x01, 0x10, 0x01, 0x10, 0x02],
        vec![0x0a, 0x01, 0x01],
        vec![0x08, 0x01, 0x12, 0x01, 0x01],
        vec![0x08],
        vec![0x08, 0x80],
        vec![
            0x08, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02,
        ],
        vec![
            0x08, 0x01, 0x10, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02,
        ],
        vec![0x08, 0x01, 0x1a, 0x02, 0x01], // truncated unknown bytes
    ] {
        assert_eq!(
            decode_wire_envelope(&test_envelope(6, &payload)),
            Err(FoundationProtocolError::MalformedEnvelope),
            "payload: {payload:?}"
        );
    }
}

#[test]
fn liveness_probe_and_ack_exact_wire_codec() -> Result<(), FoundationProtocolError> {
    let probe = [0x08, 0x05, 0x10, 0x07, 0x22, 0x03, 0x08, 0x96, 0x01];
    assert_eq!(encode_liveness_probe(7, 150)?, probe);
    let outgoing = decode_wire_envelope(&probe)?;
    assert_eq!(outgoing.message_type(), MessageType::LivenessProbe);
    assert_eq!(outgoing.validate(Direction::ServerToClient, true), Ok(()));
    assert_eq!(outgoing.server_sequence(), 0);
    assert_eq!(outgoing.payload(), &[0x08, 0x96, 0x01]);

    // Explicit diagnostic zero remains present, distinct from omission.
    let ack = [
        0x08, 0x06, 0x10, 0x07, 0x22, 0x05, 0x08, 0x96, 0x01, 0x10, 0x00,
    ];
    assert_eq!(
        decode_wire_envelope(&ack)?.liveness_ack(7)?,
        LivenessAckView {
            probe_id: 150,
            last_applied_server_sequence: Some(0),
        }
    );
    let omitted = [0x08, 0x06, 0x10, 0x07, 0x22, 0x02, 0x08, 0x01];
    assert_eq!(
        decode_wire_envelope(&omitted)?.liveness_ack(7)?,
        LivenessAckView {
            probe_id: 1,
            last_applied_server_sequence: None,
        }
    );
    assert_eq!(
        encode_liveness_probe(0, 1),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    assert_eq!(
        encode_liveness_probe(1, 0),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    Ok(())
}

#[test]
fn liveness_ack_extraction_rejects_wrong_direction_and_generation()
-> Result<(), FoundationProtocolError> {
    let ack = [0x08, 0x06, 0x10, 0x07, 0x22, 0x02, 0x08, 0x01];
    let decoded = decode_wire_envelope(&ack)?;
    assert_eq!(
        decoded.liveness_ack(8),
        Err(FoundationProtocolError::StaleConnectionGeneration)
    );
    assert_eq!(
        decoded.liveness_ack(0),
        Err(FoundationProtocolError::StaleConnectionGeneration)
    );
    assert_eq!(
        decode_wire_envelope(&[0x08, 0x06, 0x22, 0x02, 0x08, 0x01])?.liveness_ack(7),
        Err(FoundationProtocolError::StaleConnectionGeneration)
    );
    assert_eq!(
        decode_wire_envelope(&[0x08, 0x06, 0x10, 0x07, 0x18, 0x01, 0x22, 0x02, 0x08, 0x01])?
            .liveness_ack(7),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    assert_eq!(
        decode_wire_envelope(&encode_liveness_probe(7, 1)?)?.liveness_ack(7),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    for payload in [
        &[0x08, 0x01, 0x08, 0x02][..],
        &[0x08, 0x01, 0x10, 0x00, 0x10, 0x01],
        &[0x0a, 0x01, 0x01],
        &[0x08, 0x01, 0x12, 0x01, 0x01],
        &[0x08, 0x80],
        &[0x08, 0x01, 0x10, 0x80],
        &[
            0x08, 0x01, 0x10, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02,
        ],
    ] {
        assert_eq!(
            decode_wire_envelope(&test_envelope(6, payload)),
            Err(FoundationProtocolError::MalformedEnvelope),
            "payload: {payload:?}"
        );
    }
    Ok(())
}

#[test]
fn resync_domain_count_is_bounded_and_unique() -> Result<(), FoundationProtocolError> {
    let mut payload = Vec::new();
    for domain in 1..=256usize {
        let revision = test_state_revision(domain);
        payload.push(0x12);
        payload.extend(test_varint(revision.len()));
        payload.extend(revision);
    }
    assert!(decode_wire_envelope(&test_envelope(10, &payload)).is_ok());

    let revision = test_state_revision(257);
    payload.push(0x12);
    payload.extend(test_varint(revision.len()));
    payload.extend(revision);
    assert_eq!(
        decode_wire_envelope(&test_envelope(10, &payload)),
        Err(FoundationProtocolError::PayloadLimitExceeded)
    );

    let revision = test_state_revision(1);
    let mut duplicate = Vec::new();
    for _ in 0..2 {
        duplicate.push(0x12);
        duplicate.extend(test_varint(revision.len()));
        duplicate.extend_from_slice(&revision);
    }
    assert_eq!(
        decode_wire_envelope(&test_envelope(10, &duplicate)),
        Err(FoundationProtocolError::StateRevisionMismatch)
    );
    Ok(())
}

#[test]
fn nested_admission_material_limits_accept_16384_and_reject_16385() {
    let id = test_uuid_v7(1);
    let accepted_material = vec![0u8; 16_384];
    let rejected_material = vec![0u8; 16_385];

    let accepted_bootstrap = test_client_bootstrap_payload_with_material_and_build(
        1,
        1,
        1,
        &id,
        &[],
        &accepted_material,
        b"test-client",
    );
    assert!(decode_wire_envelope(&test_envelope(1, &accepted_bootstrap)).is_ok());
    let rejected_bootstrap = test_client_bootstrap_payload_with_material_and_build(
        1,
        1,
        1,
        &id,
        &[],
        &rejected_material,
        b"test-client",
    );
    assert_eq!(
        decode_wire_envelope(&test_envelope(1, &rejected_bootstrap)),
        Err(FoundationProtocolError::BootstrapLimitExceeded)
    );

    let accepted_resume =
        test_client_resume_payload_with_material(1, 1, 1, &id, &[], &accepted_material);
    assert!(decode_wire_envelope(&test_envelope(3, &accepted_resume)).is_ok());
    let rejected_resume =
        test_client_resume_payload_with_material(1, 1, 1, &id, &[], &rejected_material);
    assert_eq!(
        decode_wire_envelope(&test_envelope(3, &rejected_resume)),
        Err(FoundationProtocolError::BootstrapLimitExceeded)
    );
}

#[test]
fn bootstrap_payload_limit_accepts_65536_and_rejects_65537() -> Result<(), FoundationProtocolError>
{
    // The exact-boundary payload remains semantically valid and uses field 16
    // as an additive unknown bytes field to reach exactly 65,536 bytes.
    let id = test_uuid_v7(1);
    let mut accepted_payload = test_client_bootstrap_payload(1, 1, 1, &id, &[]);
    let unknown_bytes = 65_536 - accepted_payload.len() - 5;
    accepted_payload.extend([0x82, 0x01]);
    accepted_payload.extend(test_varint(unknown_bytes));
    accepted_payload.resize(65_536, 0);
    let accepted = test_envelope(1, &accepted_payload);
    assert_eq!(decode_wire_envelope(&accepted)?.payload().len(), 65_536);

    let mut rejected = vec![0x08, 0x01, 0x22, 0x81, 0x80, 0x04];
    rejected.resize(6 + 65_537, 0);
    assert_eq!(
        decode_wire_envelope(&rejected),
        Err(FoundationProtocolError::BootstrapLimitExceeded)
    );

    let mut resume = vec![0x08, 0x03, 0x22, 0x81, 0x80, 0x04];
    resume.resize(6 + 65_537, 0);
    assert_eq!(
        decode_wire_envelope(&resume),
        Err(FoundationProtocolError::BootstrapLimitExceeded)
    );
    Ok(())
}
