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
