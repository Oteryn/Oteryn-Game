//! Protocol/runtime authority primitives for the native Oteryn game server.

mod admission {
    include!("admission.rs");
    include!("admission_recovery_inner.rs");
}
mod admission_facade;
pub mod fnd04_verifier;
mod protocol;
mod snapshot_facade;
pub use admission::*;
pub use admission_facade::{
    AdmissionAuthority, ReconnectAttemptAuthoritySnapshot, ReconnectAttemptJournal,
};
pub use fnd04_verifier::{
    Fnd04ConsumerError, Fnd04EvidenceAuthority, Fnd04EvidenceError, Fnd04EvidenceScope,
    FreshCurrentEvidence, FreshTrustContext, NumericDate, NumericDateError,
    RecoveryCurrentEvidence, RecoveryTrustContext, VerifiedRecoveryFacts, verify_fresh_grant,
    verify_recovery_grant,
};
pub use protocol::*;
pub use snapshot_facade::SnapshotBarrier;

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const MAX_WIRE_FRAME_BYTES: u32 = 1_048_576;
pub const MAX_OUTSTANDING_COMMANDS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FoundationProtocolError {
    MalformedFrame = 1001,
    FrameTooLarge = 1002,
    MalformedEnvelope = 1003,
    UnknownMessageType = 1004,
    ProtocolMajorMismatch = 1005,
    TransportProfileMismatch = 1006,
    CapabilityMismatch = 1007,
    InvalidWireIdentifier = 1008,
    PayloadLimitExceeded = 1009,
    StaleConnectionGeneration = 1010,
    CommandOutcomeExpired = 1020,
    CommandSequenceGap = 1021,
    TooManyOutstandingCommands = 1022,
    ServerSequenceGap = 1030,
    StateRevisionMismatch = 1031,
    SnapshotAssemblyInvalid = 1032,
    SnapshotLimitExceeded = 1033,
    BootstrapLimitExceeded = 1040,
    InvalidCapabilitySet = 1041,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ProtocolDisposition {
    OperationTerminal = 1,
    ResyncRequired = 2,
    SessionFatal = 3,
    TransportFatal = 4,
}

impl FoundationProtocolError {
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }

    #[must_use]
    pub const fn disposition(self) -> ProtocolDisposition {
        match self {
            Self::MalformedFrame
            | Self::FrameTooLarge
            | Self::MalformedEnvelope
            | Self::StaleConnectionGeneration => ProtocolDisposition::TransportFatal,
            Self::UnknownMessageType
            | Self::ProtocolMajorMismatch
            | Self::TransportProfileMismatch
            | Self::CapabilityMismatch
            | Self::InvalidWireIdentifier
            | Self::SnapshotLimitExceeded
            | Self::BootstrapLimitExceeded
            | Self::InvalidCapabilitySet => ProtocolDisposition::SessionFatal,
            Self::PayloadLimitExceeded | Self::TooManyOutstandingCommands => {
                ProtocolDisposition::OperationTerminal
            }
            Self::CommandOutcomeExpired
            | Self::CommandSequenceGap
            | Self::ServerSequenceGap
            | Self::StateRevisionMismatch
            | Self::SnapshotAssemblyInvalid => ProtocolDisposition::ResyncRequired,
        }
    }
}

impl Display for FoundationProtocolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MalformedFrame => "malformed protocol frame",
            Self::FrameTooLarge => "protocol frame exceeds hard limit",
            Self::MalformedEnvelope => "malformed protocol envelope",
            Self::UnknownMessageType => "unknown foundation message type",
            Self::ProtocolMajorMismatch => "protocol major mismatch",
            Self::TransportProfileMismatch => "transport profile mismatch",
            Self::CapabilityMismatch => "capability mismatch",
            Self::InvalidWireIdentifier => "invalid wire identifier",
            Self::PayloadLimitExceeded => "payload exceeds hard limit",
            Self::StaleConnectionGeneration => "connection generation is stale",
            Self::CommandOutcomeExpired => "command outcome is no longer retained",
            Self::CommandSequenceGap => "command sequence contains a gap",
            Self::TooManyOutstandingCommands => "too many commands are outstanding",
            Self::ServerSequenceGap => "server sequence contains a gap",
            Self::StateRevisionMismatch => "state revision mismatch",
            Self::SnapshotAssemblyInvalid => "snapshot assembly is invalid",
            Self::SnapshotLimitExceeded => "snapshot exceeds hard limit",
            Self::BootstrapLimitExceeded => "bootstrap payload exceeds hard limit",
            Self::InvalidCapabilitySet => "invalid capability set",
        })
    }
}

impl Error for FoundationProtocolError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameLength(u32);

impl FrameLength {
    pub fn new(value: u32) -> Result<Self, FoundationProtocolError> {
        if value == 0 {
            return Err(FoundationProtocolError::MalformedFrame);
        }
        if value > MAX_WIRE_FRAME_BYTES {
            return Err(FoundationProtocolError::FrameTooLarge);
        }
        Ok(Self(value))
    }

    pub fn from_prefix(prefix: &[u8]) -> Result<Self, FoundationProtocolError> {
        let bytes: [u8; 4] = prefix
            .try_into()
            .map_err(|_error| FoundationProtocolError::MalformedFrame)?;
        Self::new(u32::from_be_bytes(bytes))
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn to_prefix(self) -> [u8; 4] {
        self.0.to_be_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandIdError {
    Zero,
}

impl Display for CommandIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("CommandId must be non-zero")
    }
}

impl Error for CommandIdError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId(u64);

impl CommandId {
    pub fn new(value: u64) -> Result<Self, CommandIdError> {
        if value == 0 {
            return Err(CommandIdError::Zero);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[must_use]
    const fn checked_successor(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngressDecision {
    Reserved(CommandId),
    AlreadyReserved(CommandId),
    SequenceGap { expected: CommandId },
    TooManyOutstanding { expected: CommandId },
    CommandSpaceExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateDisposition {
    NotDuplicate,
    PendingOriginal,
    ReplayRetainedOutcome,
    OutcomeExpired,
}
#[derive(Debug, Clone)]
pub struct CommandIngress {
    next_command_id: Option<CommandId>,
    pending: BTreeSet<CommandId>,
}

impl Default for CommandIngress {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandIngress {
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_command_id: Some(CommandId(1)),
            pending: BTreeSet::new(),
        }
    }

    #[must_use]
    pub const fn next_command_id(&self) -> Option<CommandId> {
        self.next_command_id
    }

    #[must_use]
    pub fn outstanding(&self) -> usize {
        self.pending.len()
    }

    pub fn reserve(&mut self, command_id: CommandId) -> IngressDecision {
        let Some(expected) = self.next_command_id else {
            return IngressDecision::CommandSpaceExhausted;
        };
        if command_id < expected {
            return IngressDecision::AlreadyReserved(command_id);
        }
        if command_id > expected {
            return IngressDecision::SequenceGap { expected };
        }
        if self.pending.len() >= MAX_OUTSTANDING_COMMANDS {
            return IngressDecision::TooManyOutstanding { expected };
        }
        let inserted = self.pending.insert(command_id);
        debug_assert!(inserted, "next CommandId cannot already be pending");
        self.next_command_id = command_id.checked_successor();
        IngressDecision::Reserved(command_id)
    }

    pub fn mark_terminal(&mut self, command_id: CommandId) -> bool {
        if self.pending.first().copied() != Some(command_id) {
            return false;
        }
        self.pending.remove(&command_id)
    }

    #[must_use]
    pub fn classify_duplicate(
        &self,
        command_id: CommandId,
        terminal_outcome_retained: bool,
    ) -> DuplicateDisposition {
        if let Some(expected) = self.next_command_id
            && command_id >= expected
        {
            return DuplicateDisposition::NotDuplicate;
        }
        if self.pending.contains(&command_id) {
            DuplicateDisposition::PendingOriginal
        } else if terminal_outcome_retained {
            DuplicateDisposition::ReplayRetainedOutcome
        } else {
            DuplicateDisposition::OutcomeExpired
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationError {
    Zero,
    StaleGeneration,
    NotNewer,
    Exhausted,
}

impl Display for GenerationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Zero => "generation or ordinal must be non-zero",
            Self::StaleGeneration => "generation is stale",
            Self::NotNewer => "external ownership grant is not newer",
            Self::Exhausted => "generation or ordinal space is exhausted",
        })
    }
}

impl Error for GenerationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionGeneration(u64);

impl ConnectionGeneration {
    pub fn new(value: u64) -> Result<Self, GenerationError> {
        if value == 0 {
            return Err(GenerationError::Zero);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    fn checked_successor(self) -> Result<Self, GenerationError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(GenerationError::Exhausted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionFence {
    current: ConnectionGeneration,
}

impl ConnectionFence {
    #[must_use]
    pub const fn fresh_admission() -> Self {
        Self {
            current: ConnectionGeneration(1),
        }
    }

    #[must_use]
    pub const fn current(self) -> ConnectionGeneration {
        self.current
    }

    #[must_use]
    pub const fn accepts(self, generation: ConnectionGeneration) -> bool {
        generation.0 == self.current.0
    }

    pub fn rebind(
        &mut self,
        predecessor: ConnectionGeneration,
    ) -> Result<ConnectionGeneration, GenerationError> {
        if predecessor != self.current {
            return Err(GenerationError::StaleGeneration);
        }
        let successor = predecessor.checked_successor()?;
        self.current = successor;
        Ok(successor)
    }

    pub(crate) fn commit_prevalidated_successor(&mut self, successor: ConnectionGeneration) {
        debug_assert_eq!(self.current.checked_successor().ok(), Some(successor));
        self.current = successor;
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeOwnershipGeneration(u64);

impl ScopeOwnershipGeneration {
    pub fn new(value: u64) -> Result<Self, GenerationError> {
        if value == 0 {
            return Err(GenerationError::Zero);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeExecutionOrdinal(u64);

impl RuntimeExecutionOrdinal {
    pub fn new(value: u64) -> Result<Self, GenerationError> {
        if value == 0 {
            return Err(GenerationError::Zero);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeWorkStamp {
    generation: ScopeOwnershipGeneration,
    ordinal: RuntimeExecutionOrdinal,
}

impl RuntimeWorkStamp {
    #[must_use]
    pub const fn generation(self) -> ScopeOwnershipGeneration {
        self.generation
    }
    #[must_use]
    pub const fn ordinal(self) -> RuntimeExecutionOrdinal {
        self.ordinal
    }
}

/// Ordinal issuance is single-owner state for one scope ownership generation.
///
/// ```compile_fail
/// use oteryn_game_server::foundation::{ScopeOwnershipGeneration, ScopeRuntimeFence};
///
/// let generation = ScopeOwnershipGeneration::new(1).unwrap();
/// let mut owner = ScopeRuntimeFence::from_external_grant(generation);
/// let mut duplicate = owner;
/// let _ = owner.accept_input(generation);
/// let _ = duplicate.accept_input(generation);
/// ```
///
/// ```compile_fail
/// use oteryn_game_server::foundation::{ScopeOwnershipGeneration, ScopeRuntimeFence};
///
/// let generation = ScopeOwnershipGeneration::new(1).unwrap();
/// let owner = ScopeRuntimeFence::from_external_grant(generation);
/// let _duplicate = owner.clone();
/// ```
///
/// ```compile_fail
/// use oteryn_game_server::foundation::{ScopeOwnershipGeneration, ScopeRuntimeFence};
///
/// let generation = ScopeOwnershipGeneration::new(1).unwrap();
/// let _first = ScopeRuntimeFence::from_external_grant(generation);
/// let _second = ScopeRuntimeFence::from_external_grant(generation);
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct ScopeRuntimeFence {
    generation: ScopeOwnershipGeneration,
    next_ordinal: Option<u64>,
}

impl ScopeRuntimeFence {
    #[must_use]
    const fn from_external_grant(generation: ScopeOwnershipGeneration) -> Self {
        Self {
            generation,
            next_ordinal: Some(1),
        }
    }

    #[must_use]
    pub const fn generation(&self) -> ScopeOwnershipGeneration {
        self.generation
    }

    pub fn accept_input(
        &mut self,
        generation: ScopeOwnershipGeneration,
    ) -> Result<RuntimeExecutionOrdinal, GenerationError> {
        if generation != self.generation {
            return Err(GenerationError::StaleGeneration);
        }
        let raw = self.next_ordinal.ok_or(GenerationError::Exhausted)?;
        self.next_ordinal = raw.checked_add(1);
        RuntimeExecutionOrdinal::new(raw)
    }

    #[must_use]
    pub const fn stamp(&self, ordinal: RuntimeExecutionOrdinal) -> RuntimeWorkStamp {
        RuntimeWorkStamp {
            generation: self.generation,
            ordinal,
        }
    }

    #[must_use]
    pub fn accepts_stamp(&self, stamp: RuntimeWorkStamp) -> bool {
        self.next_ordinal.is_some() && stamp.generation == self.generation
    }

    fn invalidate(&mut self) {
        self.next_ordinal = None;
    }

    pub fn apply_external_grant(
        &mut self,
        generation: ScopeOwnershipGeneration,
    ) -> Result<(), GenerationError> {
        if generation <= self.generation {
            return Err(GenerationError::NotNewer);
        }
        self.generation = generation;
        self.next_ordinal = Some(1);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_frame_prefix_is_big_endian_and_bounded() -> Result<(), FoundationProtocolError> {
        let maximum = FrameLength::new(1_048_576)?;
        assert_eq!(maximum.to_prefix(), [0x00, 0x10, 0x00, 0x00]);
        assert_eq!(FrameLength::from_prefix(&maximum.to_prefix())?, maximum);
        assert_eq!(
            FrameLength::new(0),
            Err(FoundationProtocolError::MalformedFrame)
        );
        assert_eq!(
            FrameLength::new(1_048_577),
            Err(FoundationProtocolError::FrameTooLarge)
        );
        assert_eq!(
            FrameLength::from_prefix(&[0x00, 0x10, 0x00]),
            Err(FoundationProtocolError::MalformedFrame)
        );
        Ok(())
    }

    #[test]
    fn command_ingress_reserves_only_the_exact_next_id() -> Result<(), CommandIdError> {
        let mut ingress = CommandIngress::new();
        let first = CommandId::new(1)?;
        let third = CommandId::new(3)?;
        assert_eq!(ingress.reserve(first), IngressDecision::Reserved(first));
        assert_eq!(ingress.next_command_id(), Some(CommandId::new(2)?));
        assert_eq!(
            ingress.reserve(third),
            IngressDecision::SequenceGap {
                expected: CommandId::new(2)?
            }
        );
        assert_eq!(ingress.next_command_id(), Some(CommandId::new(2)?));
        Ok(())
    }
    #[test]
    fn command_window_rejects_without_consuming_then_accepts_retry() -> Result<(), CommandIdError> {
        let mut ingress = CommandIngress::new();
        for raw in 1..=64 {
            let command_id = CommandId::new(raw)?;
            assert_eq!(
                ingress.reserve(command_id),
                IngressDecision::Reserved(command_id)
            );
        }
        let sixty_fifth = CommandId::new(65)?;
        assert_eq!(
            ingress.reserve(sixty_fifth),
            IngressDecision::TooManyOutstanding {
                expected: sixty_fifth
            }
        );
        assert_eq!(ingress.next_command_id(), Some(sixty_fifth));
        assert!(ingress.mark_terminal(CommandId::new(1)?));
        assert_eq!(
            ingress.reserve(sixty_fifth),
            IngressDecision::Reserved(sixty_fifth)
        );
        Ok(())
    }

    #[test]
    fn lower_command_never_reexecutes_and_reports_duplicate_state() -> Result<(), CommandIdError> {
        let mut ingress = CommandIngress::new();
        let first = CommandId::new(1)?;
        assert_eq!(ingress.reserve(first), IngressDecision::Reserved(first));
        assert_eq!(
            ingress.classify_duplicate(first, false),
            DuplicateDisposition::PendingOriginal
        );
        assert!(ingress.mark_terminal(first));
        assert_eq!(
            ingress.classify_duplicate(first, true),
            DuplicateDisposition::ReplayRetainedOutcome
        );
        assert_eq!(
            ingress.classify_duplicate(first, false),
            DuplicateDisposition::OutcomeExpired
        );
        assert_eq!(ingress.next_command_id(), Some(CommandId::new(2)?));
        Ok(())
    }
    #[test]
    fn later_reserved_command_cannot_commit_before_earlier_command() -> Result<(), CommandIdError> {
        let mut ingress = CommandIngress::new();
        let first = CommandId::new(1)?;
        let second = CommandId::new(2)?;
        assert_eq!(ingress.reserve(first), IngressDecision::Reserved(first));
        assert_eq!(ingress.reserve(second), IngressDecision::Reserved(second));
        assert!(!ingress.mark_terminal(second));
        assert!(ingress.mark_terminal(first));
        assert!(ingress.mark_terminal(second));
        Ok(())
    }

    #[test]
    fn stale_runtime_work_stamp_is_rejected_after_owner_replacement() -> Result<(), GenerationError>
    {
        let first = ScopeOwnershipGeneration::new(3)?;
        let mut runtime = ScopeRuntimeFence::from_external_grant(first);
        let ordinal = runtime.accept_input(first)?;
        let stamp = runtime.stamp(ordinal);
        assert!(runtime.accepts_stamp(stamp));
        runtime.apply_external_grant(ScopeOwnershipGeneration::new(4)?)?;
        assert!(!runtime.accepts_stamp(stamp));
        Ok(())
    }

    #[test]
    fn reconnect_advances_generation_and_fences_stale_transport() -> Result<(), GenerationError> {
        let mut fence = ConnectionFence::fresh_admission();
        let first = ConnectionGeneration::new(1)?;
        assert_eq!(fence.current(), first);
        assert!(fence.accepts(first));
        let second = fence.rebind(first)?;
        assert_eq!(second, ConnectionGeneration::new(2)?);
        assert!(!fence.accepts(first));
        assert!(fence.accepts(second));
        assert_eq!(fence.rebind(first), Err(GenerationError::StaleGeneration));
        Ok(())
    }

    #[test]
    fn runtime_ordinal_is_scoped_to_external_ownership_generation() -> Result<(), GenerationError> {
        let first_generation = ScopeOwnershipGeneration::new(7)?;
        let mut runtime = ScopeRuntimeFence::from_external_grant(first_generation);
        assert_eq!(
            runtime.accept_input(first_generation)?,
            RuntimeExecutionOrdinal::new(1)?
        );
        assert_eq!(
            runtime.accept_input(first_generation)?,
            RuntimeExecutionOrdinal::new(2)?
        );
        let replacement_generation = ScopeOwnershipGeneration::new(11)?;
        runtime.apply_external_grant(replacement_generation)?;
        assert_eq!(
            runtime.accept_input(first_generation),
            Err(GenerationError::StaleGeneration)
        );
        assert_eq!(
            runtime.accept_input(replacement_generation)?,
            RuntimeExecutionOrdinal::new(1)?
        );
        assert_eq!(
            runtime.apply_external_grant(replacement_generation),
            Err(GenerationError::NotNewer)
        );
        Ok(())
    }
}

pub mod admission_authority_publication;
