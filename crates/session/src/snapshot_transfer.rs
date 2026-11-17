//! Bounded transfer framing shared by initial and mid-session snapshots.
use super::*;

pub(super) async fn read_body<S: SessionStream>(
    stream: &mut S,
    deadline: Duration,
    session_generation: u64,
    begin: &oteryn_protocol_oteryn::SnapshotBeginFields,
) -> Result<Vec<u8>, SessionError> {
    // Exactly `begin.chunk_count` `SnapshotChunk` frames, strictly in order: a short transfer
    // (the server stops early) surfaces here as `UnexpectedMessage` (the next frame is
    // `SnapshotCommit` instead) or as a bounded `Timeout`/`Io` (the connection stalls or closes).
    // A multi-chunk transfer may split a `SnapshotBody` protobuf field at any byte offset, not
    // necessarily a field boundary (FND-02 §16), so every chunk's raw `data` is only ever
    // concatenated here, in `chunk_index` order, into one owned buffer — never decoded on its
    // own. `assembled_bytes` is checked against `SnapshotBegin`'s declared
    // `total_encoded_bytes` with checked arithmetic, and rejected, *before* the chunk's bytes are
    // appended (before growing the assembly), so an over-declaring or over-sending peer cannot
    // grow the buffer past the declared bound first and get rejected only afterwards.
    let mut assembled_body: Vec<u8> = Vec::new();
    let mut assembled_bytes: u64 = 0;
    for expected_index in 0..begin.chunk_count {
        let chunk_frame = bounded(deadline, "SnapshotChunk", read_frame(stream)).await?;
        let chunk_envelope = decode_wire_envelope(&chunk_frame)?;
        // Post-admission server traffic, same as `SnapshotBegin` above.
        chunk_envelope.validate(Direction::ServerToClient, true)?;
        if chunk_envelope.message_type() != MessageType::SnapshotChunk {
            return Err(SessionError::UnexpectedMessage {
                expected: MessageType::SnapshotChunk,
                actual: chunk_envelope.message_type(),
            });
        }
        if chunk_envelope.connection_generation() != session_generation {
            return Err(SessionError::ConnectionGenerationMismatch {
                expected: session_generation,
                actual: chunk_envelope.connection_generation(),
            });
        }
        let chunk_snapshot_id = decode_snapshot_id(chunk_envelope.payload())?;
        if chunk_snapshot_id != begin.snapshot_id {
            return Err(SessionError::SnapshotIdMismatch {
                expected: begin.snapshot_id,
                actual: chunk_snapshot_id,
            });
        }
        let (chunk_index, chunk_data) = decode_snapshot_chunk_framing(chunk_envelope.payload())?;
        if chunk_index != expected_index {
            return Err(SessionError::ChunkIndexMismatch {
                expected: expected_index,
                actual: chunk_index,
            });
        }
        let chunk_len = u64::try_from(chunk_data.len()).unwrap_or(u64::MAX);
        assembled_bytes = assembled_bytes
            .checked_add(chunk_len)
            .filter(|total| *total <= begin.total_encoded_bytes)
            .ok_or(SessionError::AssembledLengthMismatch {
                expected: begin.total_encoded_bytes,
                actual: assembled_bytes.saturating_add(chunk_len),
            })?;
        assembled_body.extend_from_slice(chunk_data);
    }
    if assembled_bytes != begin.total_encoded_bytes {
        return Err(SessionError::AssembledLengthMismatch {
            expected: begin.total_encoded_bytes,
            actual: assembled_bytes,
        });
    }

    // The commit is accepted only once every declared chunk arrived, in order, and the
    // assembled length matched.
    let commit_snapshot_id = read_snapshot_marker(
        stream,
        deadline,
        "SnapshotCommit",
        MessageType::SnapshotCommit,
        session_generation,
    )
    .await?;
    if commit_snapshot_id != begin.snapshot_id {
        return Err(SessionError::SnapshotIdMismatch {
            expected: begin.snapshot_id,
            actual: commit_snapshot_id,
        });
    }

    Ok(assembled_body)
}
