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
        })
    );
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
