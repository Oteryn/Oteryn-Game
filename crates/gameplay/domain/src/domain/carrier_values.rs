//! Immutable carrier inputs and errors shared with Content.

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
