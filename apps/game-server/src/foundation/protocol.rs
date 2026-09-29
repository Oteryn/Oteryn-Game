//! FND-02 wire envelope/framing seam.
//!
//! The wire codecs a client also needs (framing, the top-level envelope message types and their
//! encode/decode, the typed IDs, and the ALPN identifier) moved to `oteryn-protocol-oteryn`
//! (ADR-0011 §2, #162 A6, task `OTV2-20260928-protocol-oteryn-crate-c1a`): that crate has no
//! dependency on this one and is what the future native client (C1b) will also depend on. This
//! module re-exports exactly what it moved, at its original visibility, so every existing
//! consumer in this crate is unchanged. What stays here is server-only bookkeeping with no
//! consumer outside `oteryn-game-server` today: `CommandRef`/durable command correlation,
//! `ResyncPlan`/`plan_resync`, and the connection-state trackers (`StateRevisionTracker`,
//! `ServerSequenceTracker`, the local `SnapshotBarrier` wrapped by `snapshot_facade`).

use super::FoundationProtocolError;
use std::collections::BTreeMap;

pub use oteryn_protocol_oteryn::{
    ALPN_OTERYN_GAME_V1, ChannelId, CharacterId, Direction, GameSessionId,
    MAX_ADMISSION_MATERIAL_BYTES, MAX_BOOTSTRAP_PAYLOAD_BYTES, MAX_CAPABILITY_COUNT,
    MAX_CLIENT_BUILD_ID_BYTES, MAX_COMMAND_EXPECTED_REVISIONS, MAX_COMMAND_PAYLOAD_BYTES,
    MAX_COMMAND_RESULT_PAYLOAD_BYTES, MAX_ORDINARY_REPEATED_ENTRIES, MAX_RECONNECT_MATERIAL_BYTES,
    MAX_SNAPSHOT_ASSEMBLED_BYTES, MAX_SNAPSHOT_CHUNK_BYTES, MAX_SNAPSHOT_CHUNKS,
    MAX_STATE_DELTA_PAYLOAD_BYTES, MAX_STATE_DOMAINS_PER_SYNC, MessageType, NodeId,
    PROTOCOL_MAJOR_V1, Phase, Sequencing, TRANSPORT_PROFILE_TCP_TLS13_V1, WireEnvelopeView,
    WorldId, decode_framed_envelope, decode_wire_envelope,
};
// These are `pub(crate) fn`/`struct` at their origin (accessible only within this crate, same as
// before the move), but re-exported here with `pub use`: several standalone Foundation test
// binaries (`apps/game-server/tests/*_postgres.rs`, `wp5_s3b_composition.rs`) `#[path]`-include
// only `foundation/mod.rs` without `gameplay_transport` (`connection.rs`), so these have no
// in-crate consumer in that narrower compilation; `pub use` (unlike `pub(crate) use`) is exempt
// from `unused_imports` for exactly this reason, matching the `#[allow(dead_code)]` already used
// on some of these before the move.
pub(crate) use oteryn_protocol_oteryn::{
    CommandStatus, DomainSnapshot, encode_command_result, encode_single_chunk_snapshot,
    encode_state_delta,
};
pub use oteryn_protocol_oteryn::{
    ServerAcceptedValue, ServerResumeAcceptedValue, encode_command_protocol_error,
    encode_liveness_probe, encode_protocol_error, encode_server_accepted,
    encode_server_resume_accepted,
};
// `ClientBootstrapView`/`ClientResumeView`/`ClientCommandView`/`LivenessAckView` flow through
// `WireEnvelopeView`'s methods by inference; no code in this crate names them, so they are not
// re-exported. `read_varint`/`skip_field`/`bounded_length_delimited` are byte-cursor primitives
// used only by this file's own test below, which inspects an assembled snapshot chunk by hand.
#[cfg(test)]
pub(crate) use oteryn_protocol_oteryn::{bounded_length_delimited, read_varint, skip_field};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandRef {
    game_session_id: GameSessionId,
    command_id: super::CommandId,
}
impl CommandRef {
    #[must_use]
    pub const fn new(game_session_id: GameSessionId, command_id: super::CommandId) -> Self {
        Self {
            game_session_id,
            command_id,
        }
    }
    #[must_use]
    pub const fn game_session_id(self) -> GameSessionId {
        self.game_session_id
    }
    #[must_use]
    pub const fn command_id(self) -> super::CommandId {
        self.command_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResyncPlan {
    UpToDate,
    ReplayFrom(u64),
    SnapshotRequired,
}
#[must_use]
pub fn plan_resync(last_applied: u64, current: u64, retained_from: u64) -> ResyncPlan {
    if last_applied == current {
        return ResyncPlan::UpToDate;
    }
    let Some(next) = last_applied.checked_add(1) else {
        return ResyncPlan::SnapshotRequired;
    };
    if last_applied < current && next >= retained_from {
        ResyncPlan::ReplayFrom(next)
    } else {
        ResyncPlan::SnapshotRequired
    }
}
#[derive(Debug, Clone, Default)]
pub struct StateRevisionTracker {
    revisions: BTreeMap<u32, u64>,
}
impl StateRevisionTracker {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn revision(&self, domain_id: u32) -> Option<u64> {
        self.revisions.get(&domain_id).copied()
    }
    pub fn apply_delta(
        &mut self,
        domain_id: u32,
        base: u64,
        new: u64,
    ) -> Result<(), FoundationProtocolError> {
        if domain_id == 0 || new <= base {
            return Err(FoundationProtocolError::StateRevisionMismatch);
        }
        let current = self.revision(domain_id).unwrap_or(0);
        if current != base {
            return Err(FoundationProtocolError::StateRevisionMismatch);
        }
        self.revisions.insert(domain_id, new);
        Ok(())
    }
    pub fn apply_snapshot_revision(
        &mut self,
        domain_id: u32,
        revision: u64,
    ) -> Result<(), FoundationProtocolError> {
        if domain_id == 0
            || self
                .revision(domain_id)
                .is_some_and(|current| revision < current)
        {
            return Err(FoundationProtocolError::StateRevisionMismatch);
        }
        self.revisions.insert(domain_id, revision);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceDecision {
    Apply,
    Duplicate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerSequenceTracker {
    last_applied: u64,
}
impl Default for ServerSequenceTracker {
    fn default() -> Self {
        Self::new()
    }
}
impl ServerSequenceTracker {
    pub const fn new() -> Self {
        Self { last_applied: 0 }
    }
    pub const fn last_applied(&self) -> u64 {
        self.last_applied
    }
    pub fn next_expected(&self) -> Option<u64> {
        self.last_applied.checked_add(1)
    }
    pub fn observe(&self, sequence: u64) -> Result<SequenceDecision, FoundationProtocolError> {
        if sequence == 0 {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        if sequence <= self.last_applied {
            return Ok(SequenceDecision::Duplicate);
        }
        let expected = self
            .next_expected()
            .ok_or(FoundationProtocolError::ServerSequenceGap)?;
        if sequence != expected {
            return Err(FoundationProtocolError::ServerSequenceGap);
        }
        Ok(SequenceDecision::Apply)
    }
    pub fn commit_applied(&mut self, sequence: u64) -> Result<(), FoundationProtocolError> {
        let expected = self
            .next_expected()
            .ok_or(FoundationProtocolError::ServerSequenceGap)?;
        if sequence != expected {
            return Err(FoundationProtocolError::ServerSequenceGap);
        }
        self.last_applied = sequence;
        Ok(())
    }
    pub fn apply_snapshot_boundary(&mut self, target: u64) -> Result<(), FoundationProtocolError> {
        if target < self.last_applied {
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        }
        self.last_applied = target;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct SnapshotAssembly {
    id: u64,
    chunk_count: u32,
    total_bytes: u64,
    target_sequence: u64,
    generation: u64,
    chunks: BTreeMap<u32, Vec<u8>>,
    received: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotCommitResult {
    target_server_sequence: u64,
    body: Vec<u8>,
}
impl SnapshotCommitResult {
    #[must_use]
    pub const fn target_server_sequence(&self) -> u64 {
        self.target_server_sequence
    }
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

#[derive(Debug, Clone, Default)]
pub struct SnapshotBarrier {
    active: Option<SnapshotAssembly>,
    highest_snapshot_id: Option<u64>,
}
impl SnapshotBarrier {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }
    pub fn begin(
        &mut self,
        id: u64,
        chunk_count: u32,
        total_bytes: u64,
        target_sequence: u64,
        generation: u64,
    ) -> Result<(), FoundationProtocolError> {
        if id == 0
            || generation == 0
            || chunk_count > MAX_SNAPSHOT_CHUNKS
            || total_bytes > MAX_SNAPSHOT_ASSEMBLED_BYTES
        {
            return Err(FoundationProtocolError::SnapshotLimitExceeded);
        }
        if self.active.is_some()
            || self
                .highest_snapshot_id
                .is_some_and(|highest| id <= highest)
        {
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        }
        self.highest_snapshot_id = Some(id);
        self.active = Some(SnapshotAssembly {
            id,
            chunk_count,
            total_bytes,
            target_sequence,
            generation,
            chunks: BTreeMap::new(),
            received: 0,
        });
        Ok(())
    }
    pub fn chunk(
        &mut self,
        id: u64,
        index: u32,
        data: &[u8],
        generation: u64,
    ) -> Result<(), FoundationProtocolError> {
        let Some(active) = self.active.as_mut() else {
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        };
        if generation != active.generation {
            self.active = None;
            return Err(FoundationProtocolError::StaleConnectionGeneration);
        }
        if data.len() > MAX_SNAPSHOT_CHUNK_BYTES {
            self.active = None;
            return Err(FoundationProtocolError::SnapshotLimitExceeded);
        }
        if id != active.id || index >= active.chunk_count {
            self.active = None;
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        }
        if let Some(existing) = active.chunks.get(&index) {
            if existing.as_slice() == data {
                return Ok(());
            }
            self.active = None;
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        }
        let Some(next) = active.received.checked_add(data.len() as u64) else {
            self.active = None;
            return Err(FoundationProtocolError::SnapshotLimitExceeded);
        };
        if next > active.total_bytes || next > MAX_SNAPSHOT_ASSEMBLED_BYTES {
            self.active = None;
            return Err(FoundationProtocolError::SnapshotLimitExceeded);
        }
        active.received = next;
        active.chunks.insert(index, data.to_vec());
        Ok(())
    }
    pub fn commit(
        &mut self,
        id: u64,
        generation: u64,
    ) -> Result<SnapshotCommitResult, FoundationProtocolError> {
        let Some(active) = self.active.take() else {
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        };
        if generation != active.generation {
            return Err(FoundationProtocolError::StaleConnectionGeneration);
        }
        if id != active.id
            || active.received != active.total_bytes
            || active.chunks.len() != active.chunk_count as usize
            || (0..active.chunk_count).any(|i| !active.chunks.contains_key(&i))
        {
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        }
        let capacity = usize::try_from(active.total_bytes)
            .map_err(|_| FoundationProtocolError::SnapshotLimitExceeded)?;
        let mut body = Vec::with_capacity(capacity);
        for index in 0..active.chunk_count {
            let chunk = active
                .chunks
                .get(&index)
                .ok_or(FoundationProtocolError::SnapshotAssemblyInvalid)?;
            body.extend_from_slice(chunk);
        }
        Ok(SnapshotCommitResult {
            target_server_sequence: active.target_sequence,
            body,
        })
    }
    pub fn may_emit_sequenced(&self, sequence: u64, generation: u64) -> bool {
        self.active
            .as_ref()
            .is_none_or(|a| a.generation != generation || sequence <= a.target_sequence)
    }
    pub fn discard_for_generation_change(&mut self) {
        self.active = None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_ref_scopes_command_id_to_game_session() -> Result<(), FoundationProtocolError> {
        let mut one = [0u8; 16];
        one[6] = 0x70;
        one[8] = 0x80;
        one[15] = 1;
        let mut two = [0u8; 16];
        two[6] = 0x70;
        two[8] = 0x80;
        two[15] = 2;
        let command = super::super::CommandId::new(1)
            .map_err(|_| FoundationProtocolError::InvalidWireIdentifier)?;
        let a = CommandRef::new(GameSessionId::decode(&one)?, command);
        let b = CommandRef::new(GameSessionId::decode(&two)?, command);
        assert_ne!(a, b);
        assert_eq!(a.command_id(), command);
        assert_eq!(a.game_session_id().as_bytes(), &one);
        Ok(())
    }

    #[test]
    fn empty_snapshot_body_is_valid_when_encoded_length_is_zero()
    -> Result<(), FoundationProtocolError> {
        let mut barrier = SnapshotBarrier::new();
        barrier.begin(41, 0, 0, 9, 1)?;
        assert_eq!(barrier.commit(41, 1)?.target_server_sequence(), 9);
        Ok(())
    }

    #[test]
    fn server_sequence_gap_requests_resync_without_advancing() -> Result<(), FoundationProtocolError>
    {
        let mut sequence = ServerSequenceTracker::new();
        assert_eq!(sequence.observe(1)?, SequenceDecision::Apply);
        sequence.commit_applied(1)?;
        assert_eq!(sequence.observe(1)?, SequenceDecision::Duplicate);
        assert_eq!(
            sequence.observe(3),
            Err(FoundationProtocolError::ServerSequenceGap)
        );
        assert_eq!(sequence.next_expected(), Some(2));
        Ok(())
    }

    #[test]
    fn server_sequence_advances_only_after_payload_commit() -> Result<(), FoundationProtocolError> {
        let mut sequence = ServerSequenceTracker::new();
        assert_eq!(sequence.observe(1)?, SequenceDecision::Apply);
        assert_eq!(sequence.last_applied(), 0);
        assert_eq!(sequence.observe(1)?, SequenceDecision::Apply);
        sequence.commit_applied(1)?;
        assert_eq!(sequence.last_applied(), 1);
        assert_eq!(sequence.observe(1)?, SequenceDecision::Duplicate);
        Ok(())
    }

    #[test]
    fn snapshot_barrier_blocks_post_target_sequence_until_commit()
    -> Result<(), FoundationProtocolError> {
        let mut barrier = SnapshotBarrier::new();
        barrier.begin(9, 2, 4, 12, 1)?;
        assert!(!barrier.may_emit_sequenced(13, 1));
        barrier.chunk(9, 0, &[1, 2], 1)?;
        barrier.chunk(9, 1, &[3, 4], 1)?;
        let committed = barrier.commit(9, 1)?;
        assert_eq!(committed.target_server_sequence(), 12);
        assert_eq!(committed.body(), &[1, 2, 3, 4]);
        assert!(barrier.may_emit_sequenced(13, 1));
        Ok(())
    }

    #[test]
    fn snapshot_revisions_and_server_sequence_never_roll_back()
    -> Result<(), FoundationProtocolError> {
        let mut revisions = StateRevisionTracker::new();
        revisions.apply_snapshot_revision(7, 5)?;
        assert_eq!(
            revisions.apply_snapshot_revision(7, 4),
            Err(FoundationProtocolError::StateRevisionMismatch)
        );
        assert_eq!(revisions.revision(7), Some(5));

        let mut sequence = ServerSequenceTracker::new();
        assert_eq!(sequence.observe(1)?, SequenceDecision::Apply);
        sequence.commit_applied(1)?;
        assert_eq!(sequence.observe(2)?, SequenceDecision::Apply);
        sequence.commit_applied(2)?;
        assert_eq!(
            sequence.apply_snapshot_boundary(1),
            Err(FoundationProtocolError::SnapshotAssemblyInvalid)
        );
        assert_eq!(sequence.last_applied(), 2);
        Ok(())
    }

    #[test]
    fn state_revision_mismatch_never_guesses_forward() -> Result<(), FoundationProtocolError> {
        let mut revisions = StateRevisionTracker::new();
        revisions.apply_delta(7, 0, 1)?;
        assert_eq!(
            revisions.apply_delta(7, 0, 2),
            Err(FoundationProtocolError::StateRevisionMismatch)
        );
        assert_eq!(revisions.revision(7), Some(1));
        Ok(())
    }

    #[test]
    fn resync_replays_only_when_contiguous_history_is_retained() {
        assert_eq!(plan_resync(5, 8, 6), ResyncPlan::ReplayFrom(6));
        assert_eq!(plan_resync(2, 8, 6), ResyncPlan::SnapshotRequired);
        assert_eq!(plan_resync(8, 8, 6), ResyncPlan::UpToDate);
    }

    #[test]
    fn oversized_snapshot_chunk_is_a_limit_error_and_discards_assembly()
    -> Result<(), FoundationProtocolError> {
        let mut barrier = SnapshotBarrier::new();
        barrier.begin(7, 1, 524_289, 4, 1)?;
        let oversized = vec![0u8; 524_289];
        assert_eq!(
            barrier.chunk(7, 0, &oversized, 1),
            Err(FoundationProtocolError::SnapshotLimitExceeded)
        );
        assert!(!barrier.is_active());
        Ok(())
    }

    #[test]
    fn snapshot_ids_are_monotonic_across_commit_and_generation_change()
    -> Result<(), FoundationProtocolError> {
        let mut barrier = SnapshotBarrier::new();
        barrier.begin(2, 0, 0, 4, 1)?;
        barrier.commit(2, 1)?;
        assert_eq!(
            barrier.begin(2, 0, 0, 4, 1),
            Err(FoundationProtocolError::SnapshotAssemblyInvalid)
        );
        assert_eq!(
            barrier.begin(1, 0, 0, 4, 2),
            Err(FoundationProtocolError::SnapshotAssemblyInvalid)
        );
        barrier.begin(3, 1, 1, 5, 2)?;
        assert_eq!(
            barrier.chunk(3, 0, &[7], 3),
            Err(FoundationProtocolError::StaleConnectionGeneration)
        );
        assert_eq!(
            barrier.begin(2, 0, 0, 5, 3),
            Err(FoundationProtocolError::SnapshotAssemblyInvalid)
        );
        barrier.begin(4, 0, 0, 5, 3)?;
        Ok(())
    }

    #[test]
    fn generation_change_discards_partial_snapshot() -> Result<(), FoundationProtocolError> {
        let mut barrier = SnapshotBarrier::new();
        barrier.begin(1, 1, 1, 4, 1)?;
        assert_eq!(
            barrier.chunk(1, 0, &[7], 2),
            Err(FoundationProtocolError::StaleConnectionGeneration)
        );
        assert!(!barrier.is_active());
        Ok(())
    }

    #[test]
    fn first_control_server_frames_pass_foundation_ingress_validation()
    -> Result<(), FoundationProtocolError> {
        let result = encode_command_result(3, 1, 7, CommandStatus::Accepted, &[0x08, 0x01])?;
        let view = decode_wire_envelope(&result)?;
        assert_eq!(view.message_type(), MessageType::CommandResult);
        assert_eq!(
            (view.connection_generation(), view.server_sequence()),
            (3, 1)
        );
        view.validate(Direction::ServerToClient, true)?;

        let delta = encode_state_delta(3, 2, 1, 1, 2, 1, &[0x0a, 0x00])?;
        let view = decode_wire_envelope(&delta)?;
        assert_eq!(view.message_type(), MessageType::StateDelta);
        assert_eq!(view.server_sequence(), 2);
        view.validate(Direction::ServerToClient, true)?;

        let frames = encode_single_chunk_snapshot(
            3,
            1,
            0,
            &[DomainSnapshot {
                domain_id: 1,
                revision: 1,
                snapshot_type: 1,
                payload: &[0x0a, 0x00],
            }],
        )?;
        let types: Vec<MessageType> = frames
            .iter()
            .map(|frame| decode_wire_envelope(frame).map(|view| view.message_type()))
            .collect::<Result<_, _>>()?;
        assert_eq!(
            types,
            [
                MessageType::SnapshotBegin,
                MessageType::SnapshotChunk,
                MessageType::SnapshotCommit
            ]
        );
        for frame in &frames {
            let view = decode_wire_envelope(frame)?;
            assert_eq!(
                view.server_sequence(),
                0,
                "snapshot transfer is unsequenced"
            );
            view.validate(Direction::ServerToClient, true)?;
        }
        // The receiver assembles and commits the transfer (FND-02 §16).
        let chunk_view = decode_wire_envelope(&frames[1])?;
        let chunk_payload = chunk_view.payload();
        let mut cursor = 0;
        let mut data = None;
        while cursor < chunk_payload.len() {
            let key = read_varint(chunk_payload, &mut cursor)?;
            match key {
                0x1a => {
                    data = Some(bounded_length_delimited(
                        chunk_payload,
                        &mut cursor,
                        MAX_SNAPSHOT_CHUNK_BYTES,
                        FoundationProtocolError::PayloadLimitExceeded,
                    )?)
                }
                _ => skip_field(chunk_payload, &mut cursor, (key & 7) as u8)?,
            }
        }
        let data = data.ok_or(FoundationProtocolError::MalformedEnvelope)?;
        // The facade barrier also validates the committed SnapshotBody.
        let mut barrier = super::super::snapshot_facade::SnapshotBarrier::new();
        barrier.begin(1, 1, data.len() as u64, 0, 3)?;
        barrier.chunk(1, 0, data, 3)?;
        barrier.commit(1, 3)?;
        let domain = DomainSnapshot {
            domain_id: 1,
            revision: 1,
            snapshot_type: 1,
            payload: &[0x0a, 0x00],
        };
        assert!(encode_single_chunk_snapshot(3, 2, 0, &[domain, domain]).is_err());
        assert!(
            encode_single_chunk_snapshot(3, 2, 0, &vec![domain; MAX_STATE_DOMAINS_PER_SYNC + 1])
                .is_err()
        );
        // Invalid values refuse before any frame is built.
        assert!(encode_command_result(0, 1, 7, CommandStatus::Accepted, &[]).is_err());
        assert!(encode_command_result(3, 0, 7, CommandStatus::Accepted, &[]).is_err());
        assert!(encode_state_delta(3, 2, 1, 2, 2, 1, &[]).is_err());
        assert!(encode_state_delta(3, 2, 0, 1, 2, 1, &[]).is_err());
        assert!(encode_single_chunk_snapshot(3, 0, 0, &[]).is_err());
        Ok(())
    }
}
