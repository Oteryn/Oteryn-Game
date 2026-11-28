/// `decode_state_delta` round-trips through the existing server-side `encode_state_delta`.
#[test]
fn state_delta_decoder_round_trips_through_existing_server_encoder()
-> Result<(), FoundationProtocolError> {
    let wire = encode_state_delta(2, 11, 2, 40, 41, 1, &[0xaa, 0xbb])?;
    let envelope = decode_wire_envelope(&wire)?;
    assert_eq!(envelope.message_type(), MessageType::StateDelta);
    assert_eq!(envelope.validate(Direction::ServerToClient, true), Ok(()));
    assert_eq!(envelope.server_sequence(), 11);
    assert_eq!(
        decode_state_delta(envelope.payload())?,
        StateDeltaView {
            domain_id: 2,
            base_revision: 40,
            new_revision: 41,
            delta_type: 1,
            payload: &[0xaa, 0xbb],
            npc_purchase_attribution: None,
        }
    );
    Ok(())
}

#[test]
fn state_delta_decoder_applies_the_encoders_semantic_bounds() {
    // Zero domain, zero delta type, a non-advancing revision, an absent revision pair and a
    // duplicated singular field are all refused.
    for payload in [
        &[0x10, 0x01, 0x18, 0x02, 0x20, 0x01][..],
        &[0x08, 0x01, 0x10, 0x01, 0x18, 0x02],
        &[0x08, 0x01, 0x10, 0x02, 0x18, 0x02, 0x20, 0x01],
        &[0x08, 0x01, 0x10, 0x03, 0x18, 0x02, 0x20, 0x01],
        &[0x08, 0x01, 0x20, 0x01],
        &[0x08, 0x01, 0x08, 0x02, 0x10, 0x01, 0x18, 0x02, 0x20, 0x01],
    ] {
        assert_eq!(
            decode_state_delta(payload),
            Err(FoundationProtocolError::MalformedEnvelope),
            "{payload:?}"
        );
    }
    assert_eq!(
        decode_state_delta(&[0x08, 0x01, 0x18, 0x01, 0x20, 0x01]),
        Ok(StateDeltaView {
            domain_id: 1,
            base_revision: 0,
            new_revision: 1,
            delta_type: 1,
            payload: &[],
            npc_purchase_attribution: None,
        })
    );
}

#[test]
fn npc_purchase_attribution_preserves_legacy_state_delta_bytes()
-> Result<(), FoundationProtocolError> {
    let legacy_body = [0x08, 9, 0x10, 1, 0x18, 2, 0x20, 1, 0x2a, 0];
    let legacy = server_frame(MessageType::StateDelta, 3, 4, &legacy_body);
    assert_eq!(encode_state_delta(3, 4, 9, 1, 2, 1, &[])?, legacy);
    let decoded = decode_state_delta(decode_wire_envelope(&legacy)?.payload())?;
    assert!(decoded.payload.is_empty());
    assert_eq!(
        item_view::decode_character_inventory(decoded.payload),
        Ok(item_view::CharacterInventory::default())
    );
    assert_eq!(decoded.npc_purchase_attribution, None);

    let wire = encode_npc_purchase_inventory_delta(3, 4, 1, 2, &[], &[7, 1000])?;
    let envelope = decode_wire_envelope(&wire)?;
    let view = decode_state_delta(envelope.payload())?;
    assert_eq!(view.payload, decoded.payload);
    assert_eq!(
        view.npc_purchase_attribution.map(|ids| ids.command_ids().collect::<Vec<_>>()),
        Some(vec![7, 1000])
    );
    for suffix in [
        &[0x82, 1, 0][..],
        &[0x82, 1, 1, 0],
        &[0x82, 1, 2, 7, 7],
        &[0x82, 1, 1, 7, 0x82, 1, 1, 8],
    ] {
        let mut invalid = legacy_body.to_vec();
        invalid.extend(suffix);
        assert_eq!(decode_state_delta(&invalid), Err(FoundationProtocolError::MalformedEnvelope));
    }
    Ok(())
}

#[test]
fn snapshot_begin_decoder_reads_target_server_sequence_from_existing_server_encoder()
-> Result<(), FoundationProtocolError> {
    let payload = [0_u8; 3];
    for target in [0_u64, 42, u64::from(u32::MAX) + 1] {
        let frames = encode_single_chunk_snapshot(
            1,
            9,
            target,
            &[DomainSnapshot {
                domain_id: 1,
                revision: 1,
                snapshot_type: 1,
                payload: &payload,
            }],
        )?;
        let begin = decode_snapshot_begin(decode_wire_envelope(&frames[0])?.payload())?;
        assert_eq!(begin.snapshot_id, 9);
        assert_eq!(begin.target_server_sequence, target);
    }
    Ok(())
}

#[test]
fn map_replacement_snapshot_spans_bounded_chunks_and_preserves_small_transfers()
-> Result<(), FoundationProtocolError> {
    let payload = vec![0x5a; MAX_SNAPSHOT_CHUNK_BYTES + 1024];
    let domain = DomainSnapshot {
        domain_id: 17,
        revision: 3,
        snapshot_type: 1,
        payload: &payload,
    };
    let frames = encode_snapshot(2, 9, 42, &[domain])?;
    assert_eq!(frames.len(), 4);
    let begin = decode_snapshot_begin(decode_wire_envelope(&frames[0])?.payload())?;
    assert_eq!(begin.chunk_count, 2);
    assert_eq!(begin.target_server_sequence, 42);
    let mut body = Vec::new();
    for (index, frame) in frames[1..3].iter().enumerate() {
        let envelope = decode_wire_envelope(frame)?;
        assert_eq!(envelope.message_type(), MessageType::SnapshotChunk);
        assert_eq!(envelope.connection_generation(), 2);
        assert_eq!(envelope.server_sequence(), 0);
        assert_eq!(decode_snapshot_id(envelope.payload())?, 9);
        let (chunk_index, data) = decode_snapshot_chunk_framing(envelope.payload())?;
        assert_eq!(chunk_index as usize, index);
        assert!(data.len() <= MAX_SNAPSHOT_CHUNK_BYTES);
        body.extend_from_slice(data);
    }
    assert_eq!(body.len() as u64, begin.total_encoded_bytes);
    assert_eq!(decode_snapshot_body(&body)?, vec![domain]);
    let commit = decode_wire_envelope(&frames[3])?;
    assert_eq!(commit.message_type(), MessageType::SnapshotCommit);
    assert_eq!(decode_snapshot_id(commit.payload())?, 9);
    let small = DomainSnapshot {
        payload: &[1, 2, 3],
        ..domain
    };
    assert_eq!(
        encode_snapshot(2, 9, 42, &[small])?,
        encode_single_chunk_snapshot(2, 9, 42, &[small])?
    );
    assert_eq!(
        encode_single_chunk_snapshot(2, 9, 42, &[domain]),
        Err(FoundationProtocolError::PayloadLimitExceeded)
    );
    let oversized = vec![0; MAX_SNAPSHOT_ASSEMBLED_BYTES as usize];
    assert_eq!(
        encode_snapshot(
            2,
            9,
            42,
            &[DomainSnapshot {
                payload: &oversized,
                ..domain
            }]
        ),
        Err(FoundationProtocolError::PayloadLimitExceeded)
    );
    Ok(())
}

#[test]
fn snapshot_domain_fields_reject_wrong_wire_and_duplicates() {
    assert_eq!(
        decode_snapshot_body(&[0x0a, 4, 0x0a, 1, 0x18, 1]),
        Err(FoundationProtocolError::MalformedEnvelope)
    );
    for field in 1..=4 {
        for duplicate in [false, true] {
            let mut entry = Vec::new();
            for current in 1..=4 {
                if current != field || duplicate {
                    if current == 4 {
                        push_test_bytes_field(&mut entry, current, &[1]);
                    } else {
                        push_test_varint_field(&mut entry, current, 1);
                    }
                }
            }
            if (field == 4) == duplicate {
                push_test_bytes_field(&mut entry, field, &[2]);
            } else {
                push_test_varint_field(&mut entry, field, 2);
            }
            let mut body = Vec::new();
            push_test_bytes_field(&mut body, 1, &entry);
            assert_eq!(
                decode_snapshot_body(&body),
                Err(FoundationProtocolError::MalformedEnvelope),
                "field {field}, duplicate {duplicate}"
            );
        }
    }
}

#[test]
fn snapshot_id_rejects_wrong_wire_and_duplicates() {
    for extra in [&[0x08, 2][..], &[0x0a, 0]] {
        let mut payload = vec![0x08, 1];
        payload.extend(extra);
        assert_eq!(
            decode_snapshot_id(&payload),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
    }
}

#[test]
fn snapshot_begin_fields_reject_wrong_wire_and_duplicates() {
    for field in 1..=4 {
        for duplicate in [false, true] {
            let mut payload = vec![0x08, 1, 0x10, 1, 0x18, 1, 0x20, 1];
            if duplicate {
                push_test_varint_field(&mut payload, field, 2);
            } else {
                push_test_bytes_field(&mut payload, field, &[]);
            }
            assert_eq!(
                decode_snapshot_begin(&payload),
                Err(FoundationProtocolError::MalformedEnvelope),
                "field {field}, duplicate {duplicate}"
            );
            assert_eq!(
                decode_wire_envelope(&test_envelope(11, &payload)),
                Err(FoundationProtocolError::MalformedEnvelope)
            );
        }
    }
}

#[test]
fn snapshot_chunk_fields_reject_wrong_wire_and_duplicates() {
    for field in 1..=3 {
        for duplicate in [false, true] {
            let mut payload = vec![0x08, 1, 0x10, 1, 0x1a, 1, 0];
            if (field == 3) == duplicate {
                push_test_bytes_field(&mut payload, field, &[2]);
            } else {
                push_test_varint_field(&mut payload, field, 2);
            }
            assert_eq!(
                decode_snapshot_chunk_framing(&payload),
                Err(FoundationProtocolError::MalformedEnvelope),
                "field {field}, duplicate {duplicate}"
            );
            assert_eq!(
                decode_wire_envelope(&test_envelope(12, &payload)),
                Err(FoundationProtocolError::MalformedEnvelope)
            );
        }
    }
}

#[test]
fn state_revision_fields_reject_wrong_wire_and_duplicates() {
    for field in 1..=2 {
        for duplicate in [false, true] {
            let mut revision = test_state_revision(1);
            if duplicate {
                push_test_varint_field(&mut revision, field, 2);
            } else {
                push_test_bytes_field(&mut revision, field, &[]);
            }
            for (message, revisions_field) in [(7, 3), (8, 4), (10, 2)] {
                let mut payload = Vec::new();
                push_test_bytes_field(&mut payload, revisions_field, &revision);
                assert_eq!(
                    decode_wire_envelope(&test_envelope(message, &payload)),
                    Err(FoundationProtocolError::MalformedEnvelope),
                    "message {message}, field {field}, duplicate {duplicate}"
                );
            }
        }
    }
}
