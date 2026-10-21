//! Immutable carrier inputs and errors shared with Content.

use super::WorldId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierError {
    InvalidCapacity,
    CapacityArithmeticOverflow,
    AllocationFailed,
    CapacityExceeded,
    InvalidAssignmentBinding,
    ContentPinWorldMismatch,
    PlayerReservationMismatch,
    WrongScope,
    InvalidActorIdentity,
    StaleActorGeneration,
    ActorGenerationExhausted,
    InjectedAdmissionFailure,
    NamespaceAlreadyClaimed,
    ContinuityGenerationNotNewer,
    PositionUnavailable,
    PositionAlreadyInitialized,
    ControlLossConflict,
    InvalidPreProductionPositionContext,
    PositionContextMismatch,
    PositionSnapshotMismatch,
    PositionRevisionExhausted,
    MovementCreatureUnavailable,
    MovementNonCardinal,
    InvalidCreatureHealth,
    InvalidCreatureTarget,
    CreatureTargetMismatch,
    NotCreature,
    SummonHasNoRewards,
    CreatureNotActionable,
    InvalidDamage,
    DamageOverflow,
    InvalidCommitBinding,
    CommitBindingTooLarge,
    CommitAllocationFailed,
    PlanConflict,
    /// D4/D141: an attributed commit's `(sequence, sub_ordinal)` is at or below its attacker's
    /// recorded high-water mark for the same `GameSessionId`. It was already resolved once
    /// (whether or not its own receipt is still retained), so it is refused, never re-applied.
    StaleAttackerSequence,
    /// D4/D141: 16 receipts are retained, none is evictable and a new distinct occurrence
    /// arrived (fail-closed, GAME-ABILITY-01 section 12). HP and every receipt stay untouched.
    DamageReceiptCapacityExceeded,
    /// D4/D141: `sub_ordinal` is at or past `ABILITY01_EFFECT_PLAN_ENTRIES_MAX`.
    SubOrdinalOutOfRange,
    /// D4/D141: an attributed command's `character_lease_generation` is lower than its
    /// attacker's recorded one, or equal with a differing `GameSessionId`. Its session was
    /// superseded, so it is refused and never applied even when its own receipt was evicted.
    SupersededAttackerSession,
    InjectedCommitFailure,
    CommittedLethalUnavailable,
    CorpseReceiptMismatch,
    CorpseProjectionConflict,
    InjectedCorpseProjectionFailure,
    InjectedCorpseResponseFailure,
    /// D2b: `reward_occurrence` was asked for a different reward principal
    /// than the one already memoized for this committed death.
    RewardPrincipalConflict,
    /// AI-2 (§4.3/§4.9): a `SpawnDefinition`'s population/placement-cells violates its own or
    /// the registered `CREATUREAI0-RL-03` points-per-source bound.
    InvalidSpawnDefinition,
    /// CREATURE-AI-1 §1.2 `PROFILE_MISSING`: a creature admitted to thinking without a
    /// behaviour profile. Never defaulted.
    ProfileMissing,
    /// CREATURE-AI-1 §1.2 `PROFILE_INVALID` (including `CREATUREAI0-RL-07`).
    ProfileInvalid,
    /// AI-2: `realize_spawn` named a `SpawnSourceId` this carrier already realized.
    DuplicateSpawnSource,
    /// AI-2: a respawn call named a `SpawnSourceId`/cell index this carrier never realized.
    UnknownSpawnSource,
    UnknownSpawnCell,
    /// CHARM-DESC-FENCE-LEASE §3 item 2: the slot already carries a write fence of another
    /// transition of this session. The caller waits for that transition's settle step.
    WriteFenceBusy,
    /// A2: the slot is already bound to a different Character lease.
    AttackerBindingConflict,
    /// A2 / FND-04B §22: a same-session continuation whose reconcile is terminal or unprovable.
    ContinuationRefused,
}

impl std::fmt::Display for CarrierError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for CarrierError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalPosition {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

/// The exact active Content generation a Channel runtime is created with (#935 "Activation issuer
/// and current pin"): scope World, activation sequence, pair digests and the source-qualified
/// frame binding digest. In production it is minted only by the Content activation path from an
/// activated native entry generation; it is fixed for the runtime's lifetime.
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "test-support"), derive(Clone))]
pub struct ChannelContentPin {
    world_id: WorldId,
    activation_sequence: u64,
    server_artifact_digest: [u8; 32],
    client_artifact_digest: [u8; 32],
    frame_binding_digest: [u8; 32],
    map_revision_digest: [u8; 32],
    /// The first-entry start cell the active generation attests as Walkable.
    entry_start: LocalPosition,
}

impl ChannelContentPin {
    pub const fn entry_start(&self) -> LocalPosition {
        self.entry_start
    }

    pub const fn map_revision_digest(&self) -> [u8; 32] {
        self.map_revision_digest
    }
    pub const fn from_activation(
        world_id: WorldId,
        activation_sequence: u64,
        server_artifact_digest: [u8; 32],
        client_artifact_digest: [u8; 32],
        frame_binding_digest: [u8; 32],
        map_revision_digest: [u8; 32],
        entry_start: (i32, i32, i16),
    ) -> Self {
        Self {
            world_id,
            activation_sequence,
            server_artifact_digest,
            client_artifact_digest,
            frame_binding_digest,
            map_revision_digest,
            entry_start: LocalPosition {
                x: entry_start.0,
                y: entry_start.1,
                floor: entry_start.2,
            },
        }
    }

    /// Synthetic pin for runtime tests that do not exercise Content activation.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn test(world_id: WorldId) -> Self {
        Self::from_activation(world_id, 1, [1; 32], [2; 32], [3; 32], [4; 32], (0, 0, 0))
    }

    pub const fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub const fn activation_sequence(&self) -> u64 {
        self.activation_sequence
    }

    pub const fn server_artifact_digest(&self) -> [u8; 32] {
        self.server_artifact_digest
    }

    pub const fn client_artifact_digest(&self) -> [u8; 32] {
        self.client_artifact_digest
    }

    pub const fn frame_binding_digest(&self) -> [u8; 32] {
        self.frame_binding_digest
    }
}
