//! Protocol/runtime authority primitives for the native Oteryn game server.

mod admission {
    include!("admission.rs");
    include!("admission_recovery_inner.rs");
}
mod admission_facade;
pub mod fnd04_verifier;
mod protocol;
#[allow(dead_code)]
mod runtime_actor_carrier;
#[cfg(test)]
#[allow(unused_imports)] // Path-included Foundation test crates have no Movement module.
pub(crate) use runtime_actor_carrier::MovementActorFixture;
#[allow(unused_imports)]
// Crate-visible seam awaits separately allocated production composition.
pub(crate) use runtime_actor_carrier::{
    CarrierError, CurrentOwnerExactActorCommit, CurrentOwnerExactActorLookup,
    CurrentOwnerMovementPosition, ExactActorRef, MovementLocalPosition, MovementPositionContext,
    MovementPositionSnapshot, OwnerDamageCommand, OwnerDamageResult,
};
#[cfg(test)]
#[allow(dead_code)]
#[allow(clippy::duplicate_mod)] // Standalone Foundation test crates lack the library root.
#[path = "../ability/mod.rs"]
mod exact_actor_test_ability;
mod snapshot_facade;
pub use admission::*;
pub use admission_facade::{
    AdmissionAuthority, DurableFreshAdmissionAuthorityV1, ReconnectAttemptAuthoritySnapshot,
    ReconnectAttemptJournal,
};
pub use fnd04_verifier::{
    Fnd04ConsumerError, Fnd04EvidenceAuthority, Fnd04EvidenceError, Fnd04EvidenceScope,
    FreshCurrentEvidence, FreshTrustContext, NumericDate, NumericDateError,
    RecoveryCurrentEvidence, RecoveryTrustContext, VerifiedRecoveryFacts, verify_fresh_grant,
    verify_recovery_grant,
};
pub use protocol::*;
pub use snapshot_facade::SnapshotBarrier;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const MAX_WIRE_FRAME_BYTES: u32 = 1_048_576;
pub const MAX_OUTSTANDING_COMMANDS: usize = 64;
pub const MAX_RETAINED_TERMINAL_RECORDS: usize = 1;
pub const MAX_RETAINED_TERMINAL_CHARGED_BYTES: u64 = 3_116;
const MAX_RETAINED_SEMANTIC_COMPONENT_BYTES: usize = 512;
const RETAINED_SEMANTIC_U64_FIELD_COUNT: u64 = 4;
const RETAINED_SEMANTIC_U64_FIELD_BYTES: u64 = 8;
const RETAINED_SEMANTIC_COMPONENT_COUNT: u64 = 6;
const RETAINED_SEMANTIC_LENGTH_PREFIX_BYTES: u64 = 2;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLifecycleError {
    EmptySemanticComponent(&'static str),
    SemanticComponentTooLarge {
        field: &'static str,
        bytes: usize,
    },
    RetainedChargeOverflow,
    RetainedChargeExceeded {
        charged_bytes: u64,
        hard_maximum: u64,
    },
    TerminalNotPending(CommandId),
    TerminalOutOfOrder {
        expected: CommandId,
        attempted: CommandId,
    },
    NonResumableMissingTerminalTruth,
    InvalidRecoveryState(&'static str),
}

impl Display for CommandLifecycleError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySemanticComponent(field) => {
                write!(formatter, "{field} must not be empty")
            }
            Self::SemanticComponentTooLarge { field, bytes } => write!(
                formatter,
                "{field} exceeds the {MAX_RETAINED_SEMANTIC_COMPONENT_BYTES}-byte bound ({bytes} bytes)"
            ),
            Self::RetainedChargeOverflow => {
                formatter.write_str("retained semantic-record charge overflow")
            }
            Self::RetainedChargeExceeded {
                charged_bytes,
                hard_maximum,
            } => write!(
                formatter,
                "retained semantic-record charge {charged_bytes} exceeds hard maximum {hard_maximum}"
            ),
            Self::TerminalNotPending(command_id) => write!(
                formatter,
                "CommandId {} is not a pending original",
                command_id.get()
            ),
            Self::TerminalOutOfOrder {
                expected,
                attempted,
            } => write!(
                formatter,
                "CommandId {} cannot terminalize before pending CommandId {}",
                attempted.get(),
                expected.get()
            ),
            Self::NonResumableMissingTerminalTruth => formatter.write_str(
                "GameSession is non-resumable because required command lifecycle truth is missing",
            ),
            Self::InvalidRecoveryState(reason) => {
                write!(
                    formatter,
                    "invalid command lifecycle recovery state: {reason}"
                )
            }
        }
    }
}

impl Error for CommandLifecycleError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BoundedSemanticComponent(Box<str>);

impl BoundedSemanticComponent {
    fn new(field: &'static str, value: &str) -> Result<Self, CommandLifecycleError> {
        let component = Self(value.to_owned().into_boxed_str());
        component.byte_len(field)?;
        Ok(component)
    }

    fn byte_len(&self, field: &'static str) -> Result<u64, CommandLifecycleError> {
        if self.0.is_empty() {
            return Err(CommandLifecycleError::EmptySemanticComponent(field));
        }
        let bytes = self.0.len();
        if bytes > MAX_RETAINED_SEMANTIC_COMPONENT_BYTES {
            return Err(CommandLifecycleError::SemanticComponentTooLarge { field, bytes });
        }
        u64::try_from(bytes).map_err(|_error| CommandLifecycleError::RetainedChargeOverflow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedSemanticIntentIdentity {
    placement: BoundedSemanticComponent,
    incarnation: u64,
    family: BoundedSemanticComponent,
    expected_revision: u64,
}

impl NormalizedSemanticIntentIdentity {
    pub fn new(
        placement: &str,
        incarnation: u64,
        family: &str,
        expected_revision: u64,
    ) -> Result<Self, CommandLifecycleError> {
        Ok(Self {
            placement: BoundedSemanticComponent::new("normalized intent placement", placement)?,
            incarnation,
            family: BoundedSemanticComponent::new("normalized intent family", family)?,
            expected_revision,
        })
    }

    fn validate(&self) -> Result<(), CommandLifecycleError> {
        self.placement.byte_len("normalized intent placement")?;
        self.family.byte_len("normalized intent family")?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedBindingIdentity {
    content_generation: BoundedSemanticComponent,
    transition_key: BoundedSemanticComponent,
}

impl RetainedBindingIdentity {
    pub fn new(
        content_generation: &str,
        transition_key: &str,
    ) -> Result<Self, CommandLifecycleError> {
        Ok(Self {
            content_generation: BoundedSemanticComponent::new(
                "active Content-generation identity",
                content_generation,
            )?,
            transition_key: BoundedSemanticComponent::new("TransitionKey", transition_key)?,
        })
    }

    fn validate(&self) -> Result<(), CommandLifecycleError> {
        self.content_generation
            .byte_len("active Content-generation identity")?;
        self.transition_key.byte_len("TransitionKey")?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSemanticIdentity {
    intent: NormalizedSemanticIntentIdentity,
    binding: RetainedBindingIdentity,
}

impl CommandSemanticIdentity {
    #[must_use]
    pub const fn new(
        intent: NormalizedSemanticIntentIdentity,
        binding: RetainedBindingIdentity,
    ) -> Self {
        Self { intent, binding }
    }

    fn validate(&self) -> Result<(), CommandLifecycleError> {
        self.intent.validate()?;
        self.binding.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSemanticOutcome {
    disposition: BoundedSemanticComponent,
    state: BoundedSemanticComponent,
    revision: u64,
}

impl TerminalSemanticOutcome {
    pub fn new(
        disposition: &str,
        state: &str,
        revision: u64,
    ) -> Result<Self, CommandLifecycleError> {
        Ok(Self {
            disposition: BoundedSemanticComponent::new(
                "terminal semantic disposition",
                disposition,
            )?,
            state: BoundedSemanticComponent::new("terminal semantic state", state)?,
            revision,
        })
    }

    fn validate(&self) -> Result<(), CommandLifecycleError> {
        self.disposition.byte_len("terminal semantic disposition")?;
        self.state.byte_len("terminal semantic state")?;
        Ok(())
    }

    #[must_use]
    pub fn disposition(&self) -> &str {
        &self.disposition.0
    }

    #[must_use]
    pub fn state(&self) -> &str {
        &self.state.0
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }
}

fn checked_retained_record_charge(
    component_lengths: [u64; 6],
) -> Result<u64, CommandLifecycleError> {
    let fixed_u64_bytes = RETAINED_SEMANTIC_U64_FIELD_COUNT
        .checked_mul(RETAINED_SEMANTIC_U64_FIELD_BYTES)
        .ok_or(CommandLifecycleError::RetainedChargeOverflow)?;
    let length_prefix_bytes = RETAINED_SEMANTIC_COMPONENT_COUNT
        .checked_mul(RETAINED_SEMANTIC_LENGTH_PREFIX_BYTES)
        .ok_or(CommandLifecycleError::RetainedChargeOverflow)?;
    let mut total = fixed_u64_bytes
        .checked_add(length_prefix_bytes)
        .ok_or(CommandLifecycleError::RetainedChargeOverflow)?;
    for length in component_lengths {
        total = total
            .checked_add(length)
            .ok_or(CommandLifecycleError::RetainedChargeOverflow)?;
    }
    Ok(total)
}

fn retained_record_charge(
    semantic: &CommandSemanticIdentity,
    outcome: &TerminalSemanticOutcome,
) -> Result<u64, CommandLifecycleError> {
    semantic.validate()?;
    outcome.validate()?;
    checked_retained_record_charge([
        semantic
            .intent
            .placement
            .byte_len("normalized intent placement")?,
        semantic
            .intent
            .family
            .byte_len("normalized intent family")?,
        semantic
            .binding
            .content_generation
            .byte_len("active Content-generation identity")?,
        semantic.binding.transition_key.byte_len("TransitionKey")?,
        outcome
            .disposition
            .byte_len("terminal semantic disposition")?,
        outcome.state.byte_len("terminal semantic state")?,
    ])
}

#[derive(Debug, PartialEq, Eq)]
pub struct RetainedTerminalRecord {
    command_id: CommandId,
    semantic: CommandSemanticIdentity,
    outcome: TerminalSemanticOutcome,
    charged_bytes: u64,
}

impl RetainedTerminalRecord {
    fn validate(&self) -> Result<(), CommandLifecycleError> {
        let charged_bytes = retained_record_charge(&self.semantic, &self.outcome)?;
        if charged_bytes != self.charged_bytes {
            return Err(CommandLifecycleError::InvalidRecoveryState(
                "retained terminal charge does not match semantic evidence",
            ));
        }
        if charged_bytes > MAX_RETAINED_TERMINAL_CHARGED_BYTES {
            return Err(CommandLifecycleError::RetainedChargeExceeded {
                charged_bytes,
                hard_maximum: MAX_RETAINED_TERMINAL_CHARGED_BYTES,
            });
        }
        Ok(())
    }

    #[must_use]
    pub const fn command_id(&self) -> CommandId {
        self.command_id
    }

    #[must_use]
    pub const fn charged_bytes(&self) -> u64 {
        self.charged_bytes
    }

    #[must_use]
    pub const fn outcome(&self) -> &TerminalSemanticOutcome {
        &self.outcome
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalRetentionPlan {
    charged_bytes: u64,
    evicted_command_id: Option<CommandId>,
}

impl TerminalRetentionPlan {
    #[must_use]
    pub const fn charged_bytes(&self) -> u64 {
        self.charged_bytes
    }

    #[must_use]
    pub const fn evicted_command_id(&self) -> Option<CommandId> {
        self.evicted_command_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateDisposition<'a> {
    NotDuplicate,
    PendingOriginal,
    ConflictChangedInput,
    ReplayRetainedOutcome(&'a TerminalSemanticOutcome),
    OutcomeExpired,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommandIngressRecoveryState {
    next_command_id: Option<CommandId>,
    pending: BTreeMap<CommandId, CommandSemanticIdentity>,
    retained_terminal: Option<RetainedTerminalRecord>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommandIngress {
    next_command_id: Option<CommandId>,
    pending: BTreeMap<CommandId, CommandSemanticIdentity>,
    retained_terminal: Option<RetainedTerminalRecord>,
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
            pending: BTreeMap::new(),
            retained_terminal: None,
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

    #[must_use]
    pub fn retained_count(&self) -> usize {
        usize::from(self.retained_terminal.is_some())
    }

    #[must_use]
    pub fn retained_charged_bytes(&self) -> u64 {
        self.retained_terminal
            .as_ref()
            .map_or(0, RetainedTerminalRecord::charged_bytes)
    }

    pub fn reserve(
        &mut self,
        command_id: CommandId,
        semantic: CommandSemanticIdentity,
    ) -> IngressDecision {
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
        let previous = self.pending.insert(command_id, semantic);
        debug_assert!(
            previous.is_none(),
            "next CommandId cannot already be pending"
        );
        self.next_command_id = command_id.checked_successor();
        IngressDecision::Reserved(command_id)
    }

    fn preflight_terminal(
        &self,
        command_id: CommandId,
        outcome: &TerminalSemanticOutcome,
    ) -> Result<TerminalRetentionPlan, CommandLifecycleError> {
        let semantic = self
            .pending
            .get(&command_id)
            .ok_or(CommandLifecycleError::TerminalNotPending(command_id))?;
        let Some((&expected, _semantic)) = self.pending.first_key_value() else {
            return Err(CommandLifecycleError::TerminalNotPending(command_id));
        };
        if expected != command_id {
            return Err(CommandLifecycleError::TerminalOutOfOrder {
                expected,
                attempted: command_id,
            });
        }

        let charged_bytes = retained_record_charge(semantic, outcome)?;
        if charged_bytes > MAX_RETAINED_TERMINAL_CHARGED_BYTES {
            return Err(CommandLifecycleError::RetainedChargeExceeded {
                charged_bytes,
                hard_maximum: MAX_RETAINED_TERMINAL_CHARGED_BYTES,
            });
        }
        let evicted_command_id = self
            .retained_terminal
            .as_ref()
            .map(RetainedTerminalRecord::command_id);
        Ok(TerminalRetentionPlan {
            charged_bytes,
            evicted_command_id,
        })
    }

    pub fn terminalize<F>(
        &mut self,
        command_id: CommandId,
        outcome: TerminalSemanticOutcome,
        gameplay_mutation: F,
    ) -> Result<TerminalRetentionPlan, CommandLifecycleError>
    where
        F: FnOnce(),
    {
        let plan = self.preflight_terminal(command_id, &outcome)?;
        gameplay_mutation();

        if let Some(evicted_command_id) = plan.evicted_command_id {
            let evicted = self.retained_terminal.take();
            debug_assert_eq!(
                evicted.as_ref().map(RetainedTerminalRecord::command_id),
                Some(evicted_command_id)
            );
        }
        let semantic = match self.pending.remove(&command_id) {
            Some(semantic) => semantic,
            None => unreachable!("successful terminal preflight requires a pending original"),
        };
        debug_assert!(self.retained_terminal.is_none());
        self.retained_terminal = Some(RetainedTerminalRecord {
            command_id,
            semantic,
            outcome,
            charged_bytes: plan.charged_bytes,
        });
        debug_assert!(self.retained_count() <= MAX_RETAINED_TERMINAL_RECORDS);
        Ok(plan)
    }

    #[must_use]
    pub fn classify_duplicate(
        &self,
        command_id: CommandId,
        semantic: &CommandSemanticIdentity,
    ) -> DuplicateDisposition<'_> {
        if let Some(expected) = self.next_command_id
            && command_id >= expected
        {
            return DuplicateDisposition::NotDuplicate;
        }
        if let Some(pending) = self.pending.get(&command_id) {
            return if pending == semantic {
                DuplicateDisposition::PendingOriginal
            } else {
                DuplicateDisposition::ConflictChangedInput
            };
        }
        if let Some(retained) = self.retained_terminal.as_ref()
            && retained.command_id == command_id
        {
            return if &retained.semantic == semantic {
                DuplicateDisposition::ReplayRetainedOutcome(retained.outcome())
            } else {
                DuplicateDisposition::ConflictChangedInput
            };
        }
        DuplicateDisposition::OutcomeExpired
    }

    #[must_use]
    pub fn into_recovery_state(self) -> CommandIngressRecoveryState {
        CommandIngressRecoveryState {
            next_command_id: self.next_command_id,
            pending: self.pending,
            retained_terminal: self.retained_terminal,
        }
    }

    pub fn recover(state: CommandIngressRecoveryState) -> Result<Self, CommandLifecycleError> {
        if state.pending.len() > MAX_OUTSTANDING_COMMANDS {
            return Err(CommandLifecycleError::InvalidRecoveryState(
                "pending command count exceeds the FND-02 outstanding-command bound",
            ));
        }

        let mut previous_pending: Option<CommandId> = None;
        for (command_id, semantic) in &state.pending {
            semantic.validate()?;
            if let Some(previous) = previous_pending
                && previous.checked_successor() != Some(*command_id)
            {
                return Err(CommandLifecycleError::InvalidRecoveryState(
                    "pending CommandIds are not contiguous",
                ));
            }
            if let Some(next_command_id) = state.next_command_id
                && *command_id >= next_command_id
            {
                return Err(CommandLifecycleError::InvalidRecoveryState(
                    "pending CommandId reaches or exceeds ingress high-water mark",
                ));
            }
            previous_pending = Some(*command_id);
        }

        if let Some(last_pending) = previous_pending {
            match state.next_command_id {
                Some(next_command_id)
                    if last_pending.checked_successor() != Some(next_command_id) =>
                {
                    return Err(CommandLifecycleError::InvalidRecoveryState(
                        "pending suffix does not end immediately below ingress high-water mark",
                    ));
                }
                None if last_pending.get() != u64::MAX => {
                    return Err(CommandLifecycleError::InvalidRecoveryState(
                        "exhausted command space does not end at u64::MAX",
                    ));
                }
                _ => {}
            }
        }

        let expected_retained_command_id =
            if let Some((&first_pending, _semantic)) = state.pending.first_key_value() {
                if first_pending.get() == 1 {
                    None
                } else {
                    Some(CommandId(first_pending.get() - 1))
                }
            } else {
                match state.next_command_id {
                    Some(next_command_id) if next_command_id.get() == 1 => None,
                    Some(next_command_id) => Some(CommandId(next_command_id.get() - 1)),
                    None => Some(CommandId(u64::MAX)),
                }
            };

        match (
            expected_retained_command_id,
            state.retained_terminal.as_ref(),
        ) {
            (Some(_expected), None) => {
                return Err(CommandLifecycleError::NonResumableMissingTerminalTruth);
            }
            (None, Some(_retained)) => {
                return Err(CommandLifecycleError::InvalidRecoveryState(
                    "retained terminal truth exists before any command could have terminalized",
                ));
            }
            (Some(expected), Some(retained)) if retained.command_id != expected => {
                return Err(CommandLifecycleError::InvalidRecoveryState(
                    "retained terminal CommandId is not the latest terminalized command",
                ));
            }
            _ => {}
        }

        if let Some(retained) = state.retained_terminal.as_ref() {
            retained.validate()?;
            if state.pending.contains_key(&retained.command_id) {
                return Err(CommandLifecycleError::InvalidRecoveryState(
                    "one CommandId cannot be both pending and retained-terminal",
                ));
            }
            if let Some(next_command_id) = state.next_command_id
                && retained.command_id >= next_command_id
            {
                return Err(CommandLifecycleError::InvalidRecoveryState(
                    "retained terminal CommandId reaches or exceeds ingress high-water mark",
                ));
            }
        }

        Ok(Self {
            next_command_id: state.next_command_id,
            pending: state.pending,
            retained_terminal: state.retained_terminal,
        })
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

    fn semantic(tag: &str) -> Result<CommandSemanticIdentity, CommandLifecycleError> {
        Ok(CommandSemanticIdentity::new(
            NormalizedSemanticIntentIdentity::new(
                &format!("oteryn:placement.{tag}"),
                1,
                &format!("oteryn:intent.{tag}"),
                0,
            )?,
            RetainedBindingIdentity::new(
                &format!("generation-{tag}"),
                &format!("oteryn:transition.{tag}"),
            )?,
        ))
    }

    fn outcome(tag: &str, revision: u64) -> Result<TerminalSemanticOutcome, CommandLifecycleError> {
        TerminalSemanticOutcome::new(
            &format!("oteryn:result.{tag}"),
            &format!("oteryn:state.{tag}"),
            revision,
        )
    }

    #[test]
    fn command_ingress_reserves_only_the_exact_next_id() -> Result<(), Box<dyn Error>> {
        let mut ingress = CommandIngress::new();
        let first = CommandId::new(1)?;
        let third = CommandId::new(3)?;
        assert_eq!(
            ingress.reserve(first, semantic("first")?),
            IngressDecision::Reserved(first)
        );
        assert_eq!(ingress.next_command_id(), Some(CommandId::new(2)?));
        assert_eq!(
            ingress.reserve(third, semantic("third")?),
            IngressDecision::SequenceGap {
                expected: CommandId::new(2)?
            }
        );
        assert_eq!(ingress.next_command_id(), Some(CommandId::new(2)?));
        Ok(())
    }

    #[test]
    fn command_window_rejects_without_consuming_then_accepts_retry() -> Result<(), Box<dyn Error>> {
        let mut ingress = CommandIngress::new();
        let pending_semantic = semantic("pending")?;
        for raw in 1..=64 {
            let command_id = CommandId::new(raw)?;
            assert_eq!(
                ingress.reserve(command_id, pending_semantic.clone()),
                IngressDecision::Reserved(command_id)
            );
        }
        let sixty_fifth = CommandId::new(65)?;
        assert_eq!(
            ingress.reserve(sixty_fifth, pending_semantic.clone()),
            IngressDecision::TooManyOutstanding {
                expected: sixty_fifth
            }
        );
        assert_eq!(ingress.next_command_id(), Some(sixty_fifth));
        ingress.terminalize(CommandId::new(1)?, outcome("first", 1)?, || {})?;
        assert_eq!(
            ingress.reserve(sixty_fifth, pending_semantic),
            IngressDecision::Reserved(sixty_fifth)
        );
        Ok(())
    }

    #[test]
    fn retained_a1_survives_later_b1_and_replays_original_outcome() -> Result<(), Box<dyn Error>> {
        let command = CommandId::new(1)?;
        let semantic_a = semantic("a-open")?;
        let semantic_b = semantic("b-close")?;
        let outcome_a = outcome("a-opened", 1)?;
        let outcome_b = outcome("b-closed", 1)?;
        let mut session_a = CommandIngress::new();
        let mut session_b = CommandIngress::new();
        let mut a_mutations = 0_u64;

        assert_eq!(
            session_a.reserve(command, semantic_a.clone()),
            IngressDecision::Reserved(command)
        );
        session_a.terminalize(command, outcome_a.clone(), || a_mutations += 1)?;
        assert_eq!(
            session_b.reserve(command, semantic_b.clone()),
            IngressDecision::Reserved(command)
        );
        session_b.terminalize(command, outcome_b, || {})?;

        let before_replay = a_mutations;
        assert_eq!(
            session_a.classify_duplicate(command, &semantic_a),
            DuplicateDisposition::ReplayRetainedOutcome(&outcome_a)
        );
        assert_eq!(a_mutations, before_replay);
        assert_eq!(session_a.retained_count(), 1);
        assert_eq!(session_b.retained_count(), 1);
        Ok(())
    }

    #[test]
    fn changed_intent_or_binding_conflicts_for_pending_and_retained() -> Result<(), Box<dyn Error>>
    {
        let mut ingress = CommandIngress::new();
        let command = CommandId::new(1)?;
        let original = CommandSemanticIdentity::new(
            NormalizedSemanticIntentIdentity::new(
                "oteryn:placement.door",
                1,
                "oteryn:intent.open",
                0,
            )?,
            RetainedBindingIdentity::new("generation-1", "oteryn:transition.open")?,
        );
        let changed_intent = CommandSemanticIdentity::new(
            NormalizedSemanticIntentIdentity::new(
                "oteryn:placement.door",
                1,
                "oteryn:intent.close",
                0,
            )?,
            original.binding.clone(),
        );
        let changed_binding = CommandSemanticIdentity::new(
            original.intent.clone(),
            RetainedBindingIdentity::new("generation-2", "oteryn:transition.open")?,
        );

        assert_eq!(
            ingress.reserve(command, original.clone()),
            IngressDecision::Reserved(command)
        );
        assert_eq!(
            ingress.classify_duplicate(command, &changed_intent),
            DuplicateDisposition::ConflictChangedInput
        );
        assert_eq!(
            ingress.classify_duplicate(command, &changed_binding),
            DuplicateDisposition::ConflictChangedInput
        );

        ingress.terminalize(command, outcome("opened", 1)?, || {})?;
        assert_eq!(
            ingress.classify_duplicate(command, &changed_intent),
            DuplicateDisposition::ConflictChangedInput
        );
        assert_eq!(
            ingress.classify_duplicate(command, &changed_binding),
            DuplicateDisposition::ConflictChangedInput
        );
        Ok(())
    }

    #[test]
    fn terminal_eviction_expires_without_reusing_high_water() -> Result<(), Box<dyn Error>> {
        let mut ingress = CommandIngress::new();
        let first = CommandId::new(1)?;
        let second = CommandId::new(2)?;
        let semantic_first = semantic("first")?;
        let semantic_second = semantic("second")?;

        ingress.reserve(first, semantic_first.clone());
        ingress.terminalize(first, outcome("first", 1)?, || {})?;
        ingress.reserve(second, semantic_second.clone());
        let plan = ingress.terminalize(second, outcome("second", 2)?, || {})?;

        assert_eq!(plan.evicted_command_id(), Some(first));
        assert_eq!(ingress.retained_count(), MAX_RETAINED_TERMINAL_RECORDS);
        assert_eq!(
            ingress.classify_duplicate(first, &semantic_first),
            DuplicateDisposition::OutcomeExpired
        );
        assert_eq!(
            ingress.reserve(first, semantic_first),
            IngressDecision::AlreadyReserved(first)
        );
        assert_eq!(ingress.next_command_id(), Some(CommandId::new(3)?));
        Ok(())
    }

    #[test]
    fn pending_original_is_never_evicted_or_reexecuted() -> Result<(), Box<dyn Error>> {
        let mut ingress = CommandIngress::new();
        let first = CommandId::new(1)?;
        let second = CommandId::new(2)?;
        let semantic_first = semantic("first")?;
        let semantic_second = semantic("second")?;

        ingress.reserve(first, semantic_first.clone());
        ingress.reserve(second, semantic_second.clone());
        assert_eq!(
            ingress.classify_duplicate(second, &semantic_second),
            DuplicateDisposition::PendingOriginal
        );
        ingress.terminalize(first, outcome("first", 1)?, || {})?;
        assert_eq!(ingress.outstanding(), 1);
        assert_eq!(
            ingress.classify_duplicate(second, &semantic_second),
            DuplicateDisposition::PendingOriginal
        );
        let plan = ingress.terminalize(second, outcome("second", 2)?, || {})?;
        assert_eq!(plan.evicted_command_id(), Some(first));
        assert_eq!(ingress.outstanding(), 0);
        Ok(())
    }

    #[test]
    fn later_terminalization_cannot_pass_earlier_pending() -> Result<(), Box<dyn Error>> {
        let mut ingress = CommandIngress::new();
        let first = CommandId::new(1)?;
        let second = CommandId::new(2)?;
        ingress.reserve(first, semantic("first")?);
        ingress.reserve(second, semantic("second")?);
        let mut mutated = false;

        let result = ingress.terminalize(second, outcome("second", 2)?, || mutated = true);
        assert_eq!(
            result,
            Err(CommandLifecycleError::TerminalOutOfOrder {
                expected: first,
                attempted: second,
            })
        );
        assert!(!mutated);
        assert_eq!(ingress.outstanding(), 2);
        assert_eq!(ingress.retained_count(), 0);
        Ok(())
    }

    #[test]
    fn retained_charge_accepts_3116_and_rejects_3117_semantics() -> Result<(), Box<dyn Error>> {
        let maximum = "x".repeat(MAX_RETAINED_SEMANTIC_COMPONENT_BYTES);
        let semantic = CommandSemanticIdentity::new(
            NormalizedSemanticIntentIdentity::new(&maximum, u64::MAX, &maximum, u64::MAX)?,
            RetainedBindingIdentity::new(&maximum, &maximum)?,
        );
        let outcome = TerminalSemanticOutcome::new(&maximum, &maximum, u64::MAX)?;
        let command = CommandId::new(1)?;
        let mut ingress = CommandIngress::new();
        ingress.reserve(command, semantic);
        let plan = ingress.terminalize(command, outcome, || {})?;

        assert_eq!(plan.charged_bytes(), MAX_RETAINED_TERMINAL_CHARGED_BYTES);
        assert_eq!(ingress.retained_charged_bytes(), 3_116);
        assert_eq!(
            checked_retained_record_charge([513, 512, 512, 512, 512, 512])?,
            3_117
        );

        let one_over = "x".repeat(MAX_RETAINED_SEMANTIC_COMPONENT_BYTES + 1);
        assert_eq!(
            NormalizedSemanticIntentIdentity::new(&one_over, 1, "family", 0),
            Err(CommandLifecycleError::SemanticComponentTooLarge {
                field: "normalized intent placement",
                bytes: 513,
            })
        );
        Ok(())
    }

    #[test]
    fn retained_charge_checked_arithmetic_rejects_overflow() {
        assert_eq!(
            checked_retained_record_charge([u64::MAX, 0, 0, 0, 0, 0]),
            Err(CommandLifecycleError::RetainedChargeOverflow)
        );
    }

    #[test]
    fn retention_failure_occurs_before_modeled_gameplay_mutation() -> Result<(), Box<dyn Error>> {
        let mut ingress = CommandIngress::new();
        let command = CommandId::new(1)?;
        ingress.reserve(command, semantic("oversized")?);
        let invalid_semantic = CommandSemanticIdentity::new(
            NormalizedSemanticIntentIdentity {
                placement: BoundedSemanticComponent(
                    "x".repeat(MAX_RETAINED_SEMANTIC_COMPONENT_BYTES + 1)
                        .into_boxed_str(),
                ),
                incarnation: 1,
                family: BoundedSemanticComponent::new("normalized intent family", "family")?,
                expected_revision: 0,
            },
            RetainedBindingIdentity::new("generation-oversized", "oteryn:transition.oversized")?,
        );
        assert!(ingress.pending.insert(command, invalid_semantic).is_some());
        let mut mutated = false;

        let result = ingress.terminalize(command, outcome("never-applied", 1)?, || {
            mutated = true;
        });
        assert_eq!(
            result,
            Err(CommandLifecycleError::SemanticComponentTooLarge {
                field: "normalized intent placement",
                bytes: 513,
            })
        );
        assert!(!mutated);
        assert_eq!(ingress.outstanding(), 1);
        assert_eq!(ingress.retained_count(), 0);
        Ok(())
    }

    #[test]
    fn recovery_preserves_retained_truth_and_missing_truth_is_non_resumable()
    -> Result<(), Box<dyn Error>> {
        let mut ingress = CommandIngress::new();
        let first = CommandId::new(1)?;
        let second = CommandId::new(2)?;
        let semantic_first = semantic("first")?;
        let semantic_second = semantic("second")?;
        let outcome_first = outcome("first", 1)?;

        ingress.reserve(first, semantic_first.clone());
        ingress.terminalize(first, outcome_first.clone(), || {})?;
        ingress.reserve(second, semantic_second.clone());

        let state = ingress.into_recovery_state();
        let recovered = CommandIngress::recover(state)?;
        assert_eq!(
            recovered.classify_duplicate(first, &semantic_first),
            DuplicateDisposition::ReplayRetainedOutcome(&outcome_first)
        );
        assert_eq!(
            recovered.classify_duplicate(second, &semantic_second),
            DuplicateDisposition::PendingOriginal
        );

        let mut missing = recovered.into_recovery_state();
        missing.retained_terminal = None;
        assert_eq!(
            CommandIngress::recover(missing),
            Err(CommandLifecycleError::NonResumableMissingTerminalTruth)
        );
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

pub mod fresh_admission_durability;
