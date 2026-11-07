#[test]
fn seam_server_encoders_match_independent_proto_vectors() -> Result<(), FoundationProtocolError> {
    // Field tags and lengths transcribed from foundation.proto, not produced
    // by the encoder or the existing test serialization helpers.
    const ACCEPTED: &[u8] = &[
        8, 2, 34, 64, 10, 16, 0, 0, 0, 0, 0, 0, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1, 18, 16, 0, 0, 0,
        0, 0, 0, 112, 0, 128, 0, 0, 0, 0, 0, 0, 2, 26, 16, 0, 0, 0, 0, 0, 0, 112, 0, 128, 0, 0, 0,
        0, 0, 0, 3, 32, 1, 48, 1, 56, 1, 64, 1, 72, 1,
    ];
    const RESUMED: &[u8] = &[
        8, 4, 34, 26, 10, 16, 0, 0, 0, 0, 0, 0, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1, 16, 2, 24, 42,
        32, 3, 40, 1,
    ];
    const ERROR: &[u8] = &[8, 14, 34, 5, 8, 235, 7, 16, 4];
    let accepted = ServerAcceptedValue {
        game_session_id: GameSessionId::decode(&test_uuid_v7(1))?,
        world_id: WorldId::decode(&test_uuid_v7(2))?,
        channel_id: ChannelId::decode(&test_uuid_v7(3))?,
        connection_generation: 1,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: 1,
        selected_capabilities: &[],
    };
    let resumed = ServerResumeAcceptedValue {
        game_session_id: accepted.game_session_id,
        connection_generation: 2,
        current_server_sequence: 42,
        next_command_id: 3,
        schema_revision: 1,
        selected_capabilities: &[],
    };
    assert_eq!(encode_server_accepted(&accepted)?, ACCEPTED);
    assert_eq!(encode_server_resume_accepted(&resumed)?, RESUMED);
    assert_eq!(
        encode_protocol_error(FoundationProtocolError::MalformedEnvelope, 0)?,
        ERROR
    );
    for (wire, kind) in [
        (ACCEPTED, MessageType::ServerAccepted),
        (RESUMED, MessageType::ServerResumeAccepted),
        (ERROR, MessageType::ProtocolError),
    ] {
        let envelope = decode_wire_envelope(wire)?;
        assert_eq!(envelope.message_type(), kind);
        assert_eq!(envelope.server_sequence(), 0);
        envelope.validate(Direction::ServerToClient, false)?;
    }
    assert!(
        encode_server_accepted(&ServerAcceptedValue {
            connection_generation: 0,
            ..accepted
        })
        .is_err()
    );
    assert!(
        encode_server_resume_accepted(&ServerResumeAcceptedValue {
            connection_generation: 0,
            ..resumed
        })
        .is_err()
    );
    assert!(
        encode_server_accepted(&ServerAcceptedValue {
            next_command_id: 0,
            ..accepted
        })
        .is_err()
    );
    assert!(
        encode_server_resume_accepted(&ServerResumeAcceptedValue {
            schema_revision: 0,
            ..resumed
        })
        .is_err()
    );
    assert!(
        encode_server_accepted(&ServerAcceptedValue {
            // 2 is reserved for PROF-WIRE-1 but not registered.
            selected_capabilities: &[2],
            ..accepted
        })
        .is_err()
    );
    assert!(
        encode_server_resume_accepted(&ServerResumeAcceptedValue {
            selected_capabilities: &[1; 129],
            ..resumed
        })
        .is_err()
    );
    let post_admission_error =
        encode_protocol_error(FoundationProtocolError::MalformedEnvelope, 7)?;
    decode_wire_envelope(&post_admission_error)?.validate(Direction::ServerToClient, true)?;
    Ok(())
}

/// `encode_client_bootstrap` round-trips through the same server-side ingress decode
/// (`WireEnvelopeView::client_bootstrap`, driven by `validate_bootstrap_ingress`) the shipped
/// server uses for every real `ClientBootstrap`: the wire format is unchanged, only a new
/// client-direction encoder was added.
#[test]
fn client_bootstrap_encoder_round_trips_through_server_ingress_decode()
-> Result<(), FoundationProtocolError> {
    let character = CharacterId::decode(&test_uuid_v7(4))?;
    let value = ClientBootstrapValue {
        schema_revision: 3,
        character_id: character,
        admission_material: b"fixture-grant",
        client_build_id: "dev-client/0.1.0",
        supported_capabilities: &[],
    };
    let wire = encode_client_bootstrap(&value)?;
    let envelope = decode_wire_envelope(&wire)?;
    assert_eq!(envelope.message_type(), MessageType::ClientBootstrap);
    let view = envelope.client_bootstrap()?;
    assert_eq!(view.protocol_major, PROTOCOL_MAJOR_V1);
    assert_eq!(view.transport_profile, TRANSPORT_PROFILE_TCP_TLS13_V1);
    assert_eq!(view.schema_revision, value.schema_revision);
    assert_eq!(view.character_id, character);
    assert_eq!(view.admission_material, value.admission_material);
    assert_eq!(view.client_build_id, value.client_build_id);
    assert!(view.supported_capabilities.is_empty());

    // Zero schema revision and empty admission material are refused before any bytes are
    // emitted, exactly like every other required field in this crate.
    assert!(
        encode_client_bootstrap(&ClientBootstrapValue {
            schema_revision: 0,
            ..value
        })
        .is_err()
    );
    assert!(
        encode_client_bootstrap(&ClientBootstrapValue {
            admission_material: &[],
            ..value
        })
        .is_err()
    );
    Ok(())
}

/// `decode_server_accepted` round-trips through the existing server-side
/// `encode_server_accepted`: same fixture, same fields, both directions of the same wire
/// format.
#[test]
fn server_accepted_decoder_round_trips_through_existing_server_encoder()
-> Result<(), FoundationProtocolError> {
    let accepted = ServerAcceptedValue {
        game_session_id: GameSessionId::decode(&test_uuid_v7(1))?,
        world_id: WorldId::decode(&test_uuid_v7(2))?,
        channel_id: ChannelId::decode(&test_uuid_v7(3))?,
        connection_generation: 1,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: 5,
        selected_capabilities: &[],
    };
    let wire = encode_server_accepted(&accepted)?;
    let envelope = decode_wire_envelope(&wire)?;
    assert_eq!(envelope.message_type(), MessageType::ServerAccepted);
    let fields = decode_server_accepted(envelope.payload())?;
    assert_eq!(fields.game_session_id, accepted.game_session_id);
    assert_eq!(fields.world_id, accepted.world_id);
    assert_eq!(fields.channel_id, accepted.channel_id);
    assert_eq!(fields.connection_generation, accepted.connection_generation);
    assert_eq!(
        fields.current_server_sequence,
        accepted.current_server_sequence
    );
    assert_eq!(fields.next_command_id, accepted.next_command_id);
    assert_eq!(fields.schema_revision, accepted.schema_revision);
    assert!(fields.selected_capabilities.is_empty());

    // Malformed ServerAccepted payloads (the same negatives `validate_server_acceptance_ingress`
    // already covers) still fail closed through the new decoder.
    assert!(decode_server_accepted(&[]).is_err());
    Ok(())
}

/// `decode_snapshot_chunk` round-trips through the existing server-side
/// `encode_single_chunk_snapshot`, recovering the exact domain entries it packed — the join
/// snapshot a client decodes right after `ServerAccepted`.
#[test]
fn snapshot_chunk_decoder_round_trips_through_existing_server_encoder()
-> Result<(), FoundationProtocolError> {
    let world_spatial = [1_u8, 2, 3];
    let world_object_overlay = [4_u8, 5];
    let frames = encode_single_chunk_snapshot(
        1,
        7,
        0,
        &[
            DomainSnapshot {
                domain_id: 1,
                revision: 1,
                snapshot_type: 1,
                payload: &world_spatial,
            },
            DomainSnapshot {
                domain_id: 2,
                revision: 0,
                snapshot_type: 1,
                payload: &world_object_overlay,
            },
        ],
    )?;
    let chunk_envelope = decode_wire_envelope(&frames[1])?;
    assert_eq!(chunk_envelope.message_type(), MessageType::SnapshotChunk);
    let (snapshot_id, domains) = decode_snapshot_chunk(chunk_envelope.payload())?;
    assert_eq!(snapshot_id, 7);
    assert_eq!(domains.len(), 2);
    assert_eq!(domains[0].domain_id, 1);
    assert_eq!(domains[0].revision, 1);
    assert_eq!(domains[0].snapshot_type, 1);
    assert_eq!(domains[0].payload, &world_spatial);
    assert_eq!(domains[1].domain_id, 2);
    assert_eq!(domains[1].revision, 0);
    assert_eq!(domains[1].snapshot_type, 1);
    assert_eq!(domains[1].payload, &world_object_overlay);
    Ok(())
}

/// One `SnapshotChunk` payload of `domain_ids.len()` minimal entries (each `snapshot_type`
/// 1, empty payload), for exercising `decode_snapshot_chunk`'s own bounds directly — a
/// malformed/adversarial peer can send more or duplicate domain IDs even though this crate's
/// own `encode_single_chunk_snapshot` never would (it already refuses to construct either).
fn snapshot_chunk_payload_with_domain_ids(domain_ids: &[u32]) -> Vec<u8> {
    let mut body = Vec::new();
    for &domain_id in domain_ids {
        let mut entry = Vec::new();
        push_scalar(&mut entry, 1, u64::from(domain_id));
        push_scalar(&mut entry, 3, 1);
        push_bytes(&mut body, 1, &entry);
    }
    let mut chunk = Vec::new();
    push_scalar(&mut chunk, 1, 1);
    push_bytes(&mut chunk, 3, &body);
    chunk
}

/// FND02-STATE-DOMAINS-PER-SYNC (`RESOURCE_LIMITS_REGISTRY.json`): 256 unique domains
/// accepted, 257 rejected.
#[test]
fn snapshot_chunk_domain_count_accepts_256_and_rejects_257() -> Result<(), FoundationProtocolError>
{
    let ids: Vec<u32> = (1..=256).collect();
    let payload_256 = snapshot_chunk_payload_with_domain_ids(&ids);
    let (snapshot_id, domains) = decode_snapshot_chunk(&payload_256)?;
    assert_eq!(snapshot_id, 1);
    assert_eq!(domains.len(), 256);

    let ids: Vec<u32> = (1..=257).collect();
    let payload_257 = snapshot_chunk_payload_with_domain_ids(&ids);
    assert_eq!(
        decode_snapshot_chunk(&payload_257),
        Err(FoundationProtocolError::PayloadLimitExceeded)
    );
    Ok(())
}

/// FND02-STATE-DOMAINS-PER-SYNC: duplicate domain IDs rejected.
/// `SnapshotBody.domain` (field 1) must be length-delimited: a varint-typed field 1 followed by
/// bytes that happen to form a valid `StateDomainSnapshot` is refused, not decoded as a domain.
#[test]
fn snapshot_body_rejects_a_domain_field_with_a_non_length_delimited_wire_type() {
    let payload = [0_u8];
    let mut entry = Vec::new();
    entry.extend_from_slice(&[0x08, 0x01, 0x10, 0x01, 0x18, 0x01, 0x22, 0x01]);
    entry.extend_from_slice(&payload);
    let mut body = vec![0x08, u8::try_from(entry.len()).unwrap_or(u8::MAX)];
    body.extend_from_slice(&entry);
    assert_eq!(
        decode_snapshot_body(&body),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
}

#[test]
fn snapshot_chunk_rejects_a_duplicate_domain_id() {
    assert_eq!(
        decode_snapshot_chunk(&snapshot_chunk_payload_with_domain_ids(&[1, 2, 1])),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
}

/// `decode_snapshot_id` round-trips through the existing server-side
/// `encode_single_chunk_snapshot`'s `SnapshotBegin` and `SnapshotCommit` frames: a client
/// correlates all three frames of one transfer by this identity.
#[test]
fn snapshot_id_decoder_round_trips_through_begin_and_commit() -> Result<(), FoundationProtocolError>
{
    let payload = [0_u8];
    let frames = encode_single_chunk_snapshot(
        1,
        9,
        0,
        &[DomainSnapshot {
            domain_id: 1,
            revision: 1,
            snapshot_type: 1,
            payload: &payload,
        }],
    )?;
    let begin = decode_wire_envelope(&frames[0])?;
    assert_eq!(begin.message_type(), MessageType::SnapshotBegin);
    assert_eq!(decode_snapshot_id(begin.payload())?, 9);
    let commit = decode_wire_envelope(&frames[2])?;
    assert_eq!(commit.message_type(), MessageType::SnapshotCommit);
    assert_eq!(decode_snapshot_id(commit.payload())?, 9);
    Ok(())
}

/// `decode_snapshot_begin` and `decode_snapshot_chunk_framing` round-trip through the
/// existing server-side `encode_single_chunk_snapshot`: its one chunk is always declared
/// `chunk_count: 1`, `chunk_index: 0` (omitted on the wire, defaulted on decode), and
/// `total_encoded_bytes` equal to that chunk's `data` length.
#[test]
fn snapshot_begin_and_chunk_framing_round_trip_through_existing_server_encoder()
-> Result<(), FoundationProtocolError> {
    let payload = [0_u8; 3];
    let frames = encode_single_chunk_snapshot(
        1,
        9,
        0,
        &[DomainSnapshot {
            domain_id: 1,
            revision: 1,
            snapshot_type: 1,
            payload: &payload,
        }],
    )?;
    let begin_envelope = decode_wire_envelope(&frames[0])?;
    let begin = decode_snapshot_begin(begin_envelope.payload())?;
    assert_eq!(begin.snapshot_id, 9);
    assert_eq!(begin.chunk_count, 1);
    assert!(begin.total_encoded_bytes > 0);

    let chunk_envelope = decode_wire_envelope(&frames[1])?;
    let (chunk_index, data) = decode_snapshot_chunk_framing(chunk_envelope.payload())?;
    assert_eq!(chunk_index, 0);
    assert_eq!(data.len() as u64, begin.total_encoded_bytes);
    assert_eq!(decode_snapshot_body(data)?.len(), 1);
    Ok(())
}

/// FND-02 (`foundation.proto`: "Zero is invalid" for `snapshot_id`): an explicit `snapshot_id`
/// of 0 is refused exactly like an absent one, everywhere it is decoded.
#[test]
fn snapshot_id_zero_is_rejected_everywhere_it_is_decoded() {
    // Field 1, wire type 0 (varint), explicit value 0 — a standard proto3 encoder never
    // emits this (this crate's own `push_scalar` skips a zero value), but an adversarial
    // peer could.
    let explicit_zero: &[u8] = &[0x08, 0x00];
    assert_eq!(
        decode_snapshot_id(explicit_zero),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    assert_eq!(
        decode_snapshot_begin(explicit_zero),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    // decode_snapshot_chunk reuses decode_snapshot_id for its own snapshot_id.
    assert_eq!(
        decode_snapshot_chunk(explicit_zero),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
}

#[test]
fn seam_typed_resume_preserves_sequence_and_identity() -> Result<(), FoundationProtocolError> {
    let id = test_uuid_v7(2);
    let mut payload = test_client_resume_payload(1, 1, 3, &id, &[8]);
    push_test_varint_field(&mut payload, 3, 42);
    let wire = test_envelope(3, &payload);
    let envelope = decode_wire_envelope(&wire)?;
    let view = envelope.client_resume()?;
    assert_eq!(view.protocol_major, 1);
    assert_eq!(view.transport_profile, 1);
    assert_eq!(view.schema_revision, 3);
    assert_eq!(view.game_session_id.as_bytes(), &id);
    assert_eq!(view.reconnect_material, &[0xbb]);
    assert_eq!(view.client_build_id, "test-client");
    assert_eq!(view.supported_capabilities, &[8]);
    assert_eq!(view.last_applied_server_sequence, 42);
    assert!(envelope.client_bootstrap().is_err());
    Ok(())
}

#[test]
fn seam_typed_extraction_rejects_bootstrap_generation_and_sequence()
-> Result<(), FoundationProtocolError> {
    for message in [1, 3] {
        let id = test_uuid_v7(1);
        let payload = if message == 1 {
            test_client_bootstrap_payload(1, 1, 1, &id, &[])
        } else {
            test_client_resume_payload(1, 1, 1, &id, &[])
        };
        for field in [2, 3] {
            let mut wire = test_envelope(message, &payload);
            push_test_varint_field(&mut wire, field, 1);
            let envelope = decode_wire_envelope(&wire)?;
            if message == 1 {
                assert!(envelope.client_bootstrap().is_err());
            } else {
                assert!(envelope.client_resume().is_err());
            }
        }
    }
    Ok(())
}

#[test]
fn frame_length_boundary_property_style() {
    for raw in [1u32, 2, 255, 65_535, 1_048_575, 1_048_576] {
        assert_eq!(FrameLength::new(raw).map(FrameLength::get), Ok(raw));
    }
    assert_eq!(
        FrameLength::new(0),
        Err(FoundationProtocolError::MalformedFrame)
    );
    assert_eq!(
        FrameLength::new(1_048_577),
        Err(FoundationProtocolError::FrameTooLarge)
    );
}
