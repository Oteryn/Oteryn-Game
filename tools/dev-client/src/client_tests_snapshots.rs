// --- Minimal test-only wire construction for the 3 fixtures below, which need a
// `SnapshotBegin` declaring more than the one chunk `encode_single_chunk_snapshot` can ever
// produce. This is fixture-only: it simulates a hypothetical/adversarial *server*, never
// anything `connect_and_join` itself sends or trusts without checking.

fn push_test_varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn push_test_scalar(output: &mut Vec<u8>, field: u64, value: u64) {
    if value != 0 {
        push_test_varint(output, field << 3);
        push_test_varint(output, value);
    }
}

fn push_test_bytes_field(output: &mut Vec<u8>, field: u64, value: &[u8]) {
    push_test_varint(output, (field << 3) | 2);
    push_test_varint(output, value.len() as u64);
    output.extend_from_slice(value);
}

fn test_server_frame(
    message_type: MessageType,
    connection_generation: u64,
    payload: &[u8],
) -> Vec<u8> {
    test_server_frame_with_sequence(message_type, connection_generation, 0, payload)
}

/// Like `test_server_frame`, but also sets the envelope's `server_sequence` (field 3) —
/// needed only by the fixture proving `WireEnvelopeView::validate` rejects a nonzero one on
/// an unsequenced (`Sequencing::None`) message type such as `SnapshotBegin`.
fn test_server_frame_with_sequence(
    message_type: MessageType,
    connection_generation: u64,
    server_sequence: u64,
    payload: &[u8],
) -> Vec<u8> {
    let mut output = Vec::new();
    push_test_scalar(&mut output, 1, message_type as u64);
    push_test_scalar(&mut output, 2, connection_generation);
    push_test_scalar(&mut output, 3, server_sequence);
    push_test_bytes_field(&mut output, 4, payload);
    output
}

fn test_snapshot_begin(snapshot_id: u64, chunk_count: u64, total_bytes: u64) -> Vec<u8> {
    let mut payload = Vec::new();
    push_test_scalar(&mut payload, 1, snapshot_id);
    push_test_scalar(&mut payload, 2, chunk_count);
    push_test_scalar(&mut payload, 3, total_bytes);
    test_server_frame(MessageType::SnapshotBegin, 1, &payload)
}

fn test_snapshot_chunk(snapshot_id: u64, chunk_index: u64, data: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    push_test_scalar(&mut payload, 1, snapshot_id);
    push_test_scalar(&mut payload, 2, chunk_index);
    push_test_bytes_field(&mut payload, 3, data);
    test_server_frame(MessageType::SnapshotChunk, 1, &payload)
}

fn test_snapshot_commit(snapshot_id: u64) -> Vec<u8> {
    let mut payload = Vec::new();
    push_test_scalar(&mut payload, 1, snapshot_id);
    test_server_frame(MessageType::SnapshotCommit, 1, &payload)
}

/// Drives `connect_and_join` against a fixture that sends exactly `frames` after
/// `ServerAccepted`, and asserts the join fails with `expected`.
async fn assert_join_fails_with(
    frames: Vec<Vec<u8>>,
    expected: impl Fn(&DevClientError) -> bool + Send + 'static,
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    let (certificate, acceptor, listener, address) = tls_test_listener().await?;
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        read_and_discard_bootstrap(&mut stream).await?;
        write_frame(&mut stream, &fake_accepted_frame()?).await?;
        for frame in &frames {
            write_frame(&mut stream, frame).await?;
        }
        stream.flush().await?;
        Ok::<(), Box<dyn StdError + Send + Sync>>(())
    });

    let character_id = CharacterId::decode(&test_uuid_v7(4))?;
    let result = connect_and_join(JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id,
        admission_material: b"fixture-grant",
        client_build_id: CLIENT_BUILD_ID,
        deadline: TEST_DEADLINE,
    })
    .await;
    let Err(error) = &result else {
        return Err(format!("expected a join failure, got {result:?}").into());
    };
    if !expected(error) {
        return Err(format!("unexpected error variant: {error:?}").into());
    }
    server.await??;
    Ok(())
}

/// Codex P2 finding 2 (#162 review on PR #1147, `f0f27df`): `SnapshotBegin` declares 2
/// chunks, but the server sends only 1 before the commit — rejected (the commit arrives
/// where a 2nd `SnapshotChunk` was expected).
#[test]
fn connect_and_join_rejects_fewer_chunks_than_snapshot_begin_declared()
-> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(assert_join_fails_with(
        vec![
            test_snapshot_begin(1, 2, 0),
            test_snapshot_chunk(1, 0, &[]),
            test_snapshot_commit(1),
        ],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::UnexpectedMessage {
                    expected: MessageType::SnapshotChunk,
                    actual: MessageType::SnapshotCommit
                })
            )
        },
    ))?
}

/// Codex P2 finding 2: a `SnapshotChunk` whose `chunk_index` does not match the index
/// expected next is rejected.
#[test]
fn connect_and_join_rejects_a_wrong_chunk_index() -> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(assert_join_fails_with(
        vec![test_snapshot_begin(1, 1, 0), test_snapshot_chunk(1, 1, &[])],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::ChunkIndexMismatch {
                    expected: 0,
                    actual: 1
                })
            )
        },
    ))?
}

/// Codex P2 finding 2: the summed `SnapshotChunk` `data` bytes must equal `SnapshotBegin`'s
/// declared `total_encoded_bytes`; a mismatch is rejected before the commit is trusted.
#[test]
fn connect_and_join_rejects_an_assembled_length_mismatch()
-> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(assert_join_fails_with(
        vec![
            // Declares 100 encoded bytes but the one chunk sent carries an empty (valid,
            // zero-domain) body: 0 actual bytes assembled.
            test_snapshot_begin(1, 1, 100),
            test_snapshot_chunk(1, 0, &[]),
            test_snapshot_commit(1),
        ],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::AssembledLengthMismatch {
                    expected: 100,
                    actual: 0
                })
            )
        },
    ))?
}

/// C1b fix round 3 finding 1 (#1147 review): a valid 2-chunk transfer whose assembled
/// `SnapshotBody` protobuf bytes are split across the chunk boundary at an arbitrary byte
/// offset — not necessarily a field boundary (FND-02 §16) — must still succeed. Proves every
/// chunk's raw `data` is concatenated, in `chunk_index` order, before the body is decoded
/// once, rather than each chunk's `data` being decoded on its own.
#[test]
fn connect_and_join_assembles_a_two_chunk_snapshot_split_mid_field()
-> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(run_two_chunk_split_case())?
}

async fn run_two_chunk_split_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
    let content_generation = [0x22_u8; 32];
    let spatial_payload = encode_world_spatial(&WireSpatialObservation {
        content_generation,
        actor_position: ActorPosition {
            x: 7,
            y: 9,
            floor: 1,
        },
    });
    let overlay_payload = encode_world_object_overlay_snapshot(&[WireOverlayEntry {
        content_generation,
        placement: DOOR_PLACEMENT.as_bytes().to_vec(),
        state: DOOR_STATE.as_bytes().to_vec(),
        revision: 3,
    }])
    .map_err(|error| format!("encode_world_object_overlay_snapshot: {error:?}"))?;
    let domains = [
        DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision: 1,
            snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            payload: &spatial_payload,
        },
        DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: 3,
            snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            payload: &overlay_payload,
        },
    ];
    // A single-chunk transfer's one chunk's `data` *is* the complete, validly-encoded
    // `SnapshotBody` (`encode_single_chunk_snapshot`'s own contract) — reused here only to
    // obtain real assembled-body bytes to split in two, never sent as a single chunk.
    let single = encode_single_chunk_snapshot(1, 7, 0, &domains)?;
    let single_chunk_envelope = decode_wire_envelope(&single[1])?;
    let (_, whole_body) = decode_snapshot_chunk_framing(single_chunk_envelope.payload())?;
    let body_len = whole_body.len() as u64;
    let split_at = whole_body.len() / 2;
    assert!(
        split_at > 0 && split_at < whole_body.len(),
        "fixture body too small to split"
    );
    let first_half = whole_body[..split_at].to_vec();
    let second_half = whole_body[split_at..].to_vec();

    let (certificate, acceptor, listener, address) = tls_test_listener().await?;
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        read_and_discard_bootstrap(&mut stream).await?;
        write_frame(&mut stream, &fake_accepted_frame()?).await?;
        write_frame(&mut stream, &test_snapshot_begin(7, 2, body_len)).await?;
        write_frame(&mut stream, &test_snapshot_chunk(7, 0, &first_half)).await?;
        write_frame(&mut stream, &test_snapshot_chunk(7, 1, &second_half)).await?;
        write_frame(&mut stream, &test_snapshot_commit(7)).await?;
        stream.flush().await?;
        Ok::<(), Box<dyn StdError + Send + Sync>>(())
    });

    let character_id = CharacterId::decode(&test_uuid_v7(4))?;
    let snapshot = connect_and_join(JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id,
        admission_material: b"fixture-grant",
        client_build_id: CLIENT_BUILD_ID,
        deadline: TEST_DEADLINE,
    })
    .await
    .map_err(|error| format!("connect_and_join: {error}"))?;
    server.await??;

    assert_eq!(
        snapshot.world_spatial.actor_position,
        ActorPosition {
            x: 7,
            y: 9,
            floor: 1
        }
    );
    assert_eq!(snapshot.world_object_overlay.len(), 1);
    assert_eq!(snapshot.world_object_overlay[0].revision, 3);
    assert_eq!(snapshot.world_spatial_revision, 1);
    assert_eq!(snapshot.world_object_overlay_revision, 3);
    Ok(())
}

/// C1b fix round 3 finding 2 (#1147 review): `ServerAccepted`'s own envelope
/// `connection_generation` must be absent (0) pre-admission (FND-02 §8/§11) — a server that
/// sends a nonzero one is rejected by `WireEnvelopeView::validate` before the payload
/// (which carries the *session's* new generation in a different field) is even decoded.
#[test]
fn connect_and_join_rejects_a_server_accepted_with_a_nonzero_envelope_generation()
-> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(run_server_accepted_nonzero_generation_case())?
}

async fn run_server_accepted_nonzero_generation_case() -> Result<(), Box<dyn StdError + Send + Sync>>
{
    let accepted_payload = decode_wire_envelope(&fake_accepted_frame()?)?
        .payload()
        .to_vec();
    let tampered = test_server_frame(MessageType::ServerAccepted, 5, &accepted_payload);

    let (certificate, acceptor, listener, address) = tls_test_listener().await?;
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        read_and_discard_bootstrap(&mut stream).await?;
        write_frame(&mut stream, &tampered).await?;
        stream.flush().await?;
        Ok::<(), Box<dyn StdError + Send + Sync>>(())
    });

    let character_id = CharacterId::decode(&test_uuid_v7(4))?;
    let result = connect_and_join(JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id,
        admission_material: b"fixture-grant",
        client_build_id: CLIENT_BUILD_ID,
        deadline: TEST_DEADLINE,
    })
    .await;
    assert!(matches!(
        result,
        Err(DevClientError::Session(SessionError::Protocol(
            FoundationProtocolError::MalformedEnvelope
        )))
    ));
    server.await??;
    Ok(())
}

/// C1b fix round 3 finding 2: `SnapshotBegin` (and every other snapshot-transfer frame) is
/// `Sequencing::None` (FND-02 §14: transfer-control frames carry no `server_sequence`); a
/// server that sets a nonzero one is rejected by `WireEnvelopeView::validate`.
#[test]
fn connect_and_join_rejects_a_snapshot_begin_with_a_nonzero_server_sequence()
-> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(assert_join_fails_with(
        vec![{
            let mut payload = Vec::new();
            push_test_scalar(&mut payload, 1, 1);
            push_test_scalar(&mut payload, 2, 1);
            push_test_scalar(&mut payload, 3, 0);
            test_server_frame_with_sequence(MessageType::SnapshotBegin, 1, 7, &payload)
        }],
        |error| {
            matches!(
                error,
                DevClientError::Session(SessionError::Protocol(
                    FoundationProtocolError::MalformedEnvelope
                ))
            )
        },
    ))?
}
