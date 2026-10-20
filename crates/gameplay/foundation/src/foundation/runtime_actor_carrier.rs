//! Fixed-bound, pre-production-only Channel actor storage.
//!
//! The module stays crate-private. Its only continuity grant is derived by
//! `ChannelRuntimeV1::from_committed_assignment` from the committed Channel
//! assignment that `serve` consumes before readiness; one runtime per
//! ownership generation is enforced by that composition, not by this module.

use super::{
    ChannelId, CharacterId, CharacterLease, CommandRef, GameSessionId, NodeId,
    ScopeOwnershipGeneration, WorldId,
};
#[path = "runtime_actor_companion.rs"]
pub mod runtime_actor_companion;
#[path = "runtime_actor_spell_types.rs"]
pub mod runtime_actor_spell_types;

#[path = "runtime_actor_familiar_defense.rs"]
pub mod runtime_actor_familiar_defense;
#[path = "runtime_actor_party.rs"]
pub mod runtime_actor_party;
#[path = "runtime_actor_periodic.rs"]
pub mod runtime_actor_periodic;
#[path = "runtime_actor_reentry.rs"]
pub mod runtime_actor_reentry;
#[path = "runtime_actor_source_reservation.rs"]
pub mod runtime_actor_source_reservation;
#[path = "runtime_actor_source_step.rs"]
pub mod runtime_actor_source_step;
#[path = "runtime_actor_spell.rs"]
pub mod runtime_actor_spell;

use std::collections::{BTreeMap, BTreeSet};
use std::mem::size_of;
use std::sync::Arc;

#[path = "runtime_actor_conditions.rs"]
mod runtime_actor_conditions;
// SPAWN-1a: the pure spawn schedule (std only) compiles with the carrier so every
// path-included Foundation crate has it without the `ai` tree.
#[path = "spawn.rs"]
mod spawn;
#[allow(unused_imports)] // The Foundation facade is composed by the owning consumer child.
pub use runtime_actor_conditions::{
    ActorConditionPlan, ActorConditionTickPlan, ActorConditionTransition, ApplicationFacts,
    AttributeModifier, AttributeModifiers, CombatSkill, ConditionDefinition, ConditionOwnerError,
    ConditionRefusal, ConditionSource, ConditionSourceKind, ConditionStore, ConditionTick,
    ConditionType, ConditionValues, DamageSchedule, DamageSegment, DotElement, ExactSpeedRatio,
    RationalSpeedRange, SpeedRange, StatusKind, TickFacts, TickKind,
};
use runtime_actor_conditions::{CreatureCommitState, PlayerRuntimeState};
#[path = "npc_actor.rs"]
pub mod npc_actor;
#[path = "runtime_actor_death.rs"]
mod runtime_actor_death;

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

/// Authority supplied by a future, independently accepted assignment consumer.
///
/// There is deliberately no constructor, issuer, reissuer, `Clone`, or `Copy`
/// implementation here. Consuming this value makes namespace bootstrap one-shot.
#[derive(Debug)]
pub struct PreProductionContinuityGrant {
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub scope_generation: ScopeOwnershipGeneration,
}

/// Surviving pre-production namespace continuity state.
///
/// This guard deliberately lives outside carrier backing and is neither
/// `Clone` nor `Copy`. A carrier claims the current generation once; losing
/// that carrier therefore cannot make the generation claimable again.
#[derive(Debug)]
pub struct NamespaceContinuityGuard {
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub current_generation: ScopeOwnershipGeneration,
    current_generation_claimed: bool,
    active: bool,
}

impl NamespaceContinuityGuard {
    pub fn from_pre_production_grant(grant: PreProductionContinuityGrant) -> Self {
        Self {
            world_id: grant.world_id,
            channel_id: grant.channel_id,
            current_generation: grant.scope_generation,
            current_generation_claimed: false,
            active: true,
        }
    }

    pub fn advance(&mut self, grant: PreProductionContinuityGrant) -> Result<(), CarrierError> {
        if grant.world_id != self.world_id || grant.channel_id != self.channel_id {
            return Err(CarrierError::WrongScope);
        }
        if grant.scope_generation.get() <= self.current_generation.get() {
            return Err(CarrierError::ContinuityGenerationNotNewer);
        }

        self.current_generation = grant.scope_generation;
        self.current_generation_claimed = false;
        self.active = true;
        Ok(())
    }

    fn claim_current_generation(&mut self) -> Result<(), CarrierError> {
        self.ensure_current_generation_unclaimed()?;
        self.current_generation_claimed = true;
        Ok(())
    }

    fn ensure_current_generation_unclaimed(&self) -> Result<(), CarrierError> {
        if !self.active {
            return Err(CarrierError::WrongScope);
        }
        if self.current_generation_claimed {
            return Err(CarrierError::NamespaceAlreadyClaimed);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorLocalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorLocalGeneration(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorRef {
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub scope_generation: ScopeOwnershipGeneration,
    pub actor_local_id: ActorLocalId,
    pub actor_local_generation: ActorLocalGeneration,
}

/// Inseparable semantic actor handle issued by this one Channel carrier.
/// Its internals are exposed within the crate for carrier qualification tests.
/// It is never decoded from a client handle or derived from the fixture Ability `TargetId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactActorRef(pub ActorRef);

impl ExactActorRef {
    pub const fn world_id(self) -> WorldId {
        self.0.world_id
    }
    pub const fn channel_id(self) -> ChannelId {
        self.0.channel_id
    }
    pub const fn scope_generation(self) -> ScopeOwnershipGeneration {
        self.0.scope_generation
    }
    pub const fn actor_local_id(self) -> u32 {
        self.0.actor_local_id.0
    }
    pub const fn actor_local_generation(self) -> u64 {
        self.0.actor_local_generation.0
    }
    /// Stable opaque placement identity: a domain-separated digest of the exact actor
    /// reference in its runtime scope. It carries no position and reveals no slot layout.
    pub fn placement_identity(self) -> [u8; 16] {
        use sha2::{Digest, Sha256};
        let ActorRef {
            world_id,
            channel_id,
            scope_generation,
            actor_local_id,
            actor_local_generation,
        } = self.0;
        let digest = Sha256::new()
            .chain_update(b"oteryn:runtime-player-placement:v1")
            .chain_update(world_id.as_bytes())
            .chain_update(channel_id.as_bytes())
            .chain_update(scope_generation.get().to_be_bytes())
            .chain_update(actor_local_id.0.to_be_bytes())
            .chain_update(actor_local_generation.0.to_be_bytes())
            .finalize();
        let mut identity = [0_u8; 16];
        identity.copy_from_slice(&digest[..16]);
        identity
    }
}

impl ExactActorRef {
    /// Transport test fixture: an actor reference that names no runtime slot.
    #[cfg(any(test, feature = "test-support"))]
    #[allow(clippy::expect_used, dead_code)]
    pub fn transport_fixture(world_id: WorldId, channel_id: ChannelId) -> Self {
        Self(ActorRef {
            world_id,
            channel_id,
            scope_generation: ScopeOwnershipGeneration::new(1).expect("generation"),
            actor_local_id: ActorLocalId(1),
            actor_local_generation: ActorLocalGeneration(1),
        })
    }
}

/// One fixed-slot reservation for a fresh GameSession. It is not a playable
/// actor until the owning durable admission is proven committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerActorReservation {
    game_session_id: GameSessionId,
    actor_ref: ActorRef,
}

/// Immutable provenance of the committed Channel assignment that created this
/// runtime. Source revision identifies the exact assignment decision; no
/// production capacity value lives here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelRuntimeAssignmentBinding {
    world_id: WorldId,
    channel_id: ChannelId,
    node_id: NodeId,
    node_registration_revision: u64,
    scope_generation: ScopeOwnershipGeneration,
    source_revision: u64,
}

impl ChannelRuntimeAssignmentBinding {
    pub const fn world_id(self) -> WorldId {
        self.world_id
    }

    pub const fn channel_id(self) -> ChannelId {
        self.channel_id
    }

    pub const fn scope_generation(self) -> ScopeOwnershipGeneration {
        self.scope_generation
    }

    /// The committed assignment decision revision this runtime was composed from.
    pub const fn source_revision(self) -> u64 {
        self.source_revision
    }

    #[allow(clippy::too_many_arguments)]
    pub fn matches_committed_assignment(
        self,
        world_id: WorldId,
        channel_id: ChannelId,
        node_id: NodeId,
        node_registration_revision: u64,
        ownership_generation: u64,
        source_revision: u64,
        decision_identity: &str,
    ) -> bool {
        let decision_revision = decision_identity
            .strip_prefix("runtime-scope-assignment:")
            .and_then(|value| value.parse::<u64>().ok());
        self.world_id == world_id
            && self.channel_id == channel_id
            && self.node_id == node_id
            && self.node_registration_revision == node_registration_revision
            && self.scope_generation.get() == ownership_generation
            && self.source_revision == source_revision
            && decision_revision == Some(source_revision)
    }
}

/// A borrow of independently current owner continuity and its matching carrier.
/// No mutation, admission or continuity-grant method crosses this boundary.
pub struct CurrentOwnerExactActorLookup<'a> {
    carrier: &'a ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
}

/// A short-lived mutable owner borrow. No resolved snapshot can grant this
/// capability: the carrier checks independent continuity and slot generation
/// again at the sole write.
pub struct CurrentOwnerExactActorCommit<'a> {
    content: Option<&'a ChannelContentPin>,
    pub carrier: &'a mut ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
    /// The owner clock (ms) of this turn, recorded on each commit's receipt and contributor.
    /// `None` records no time: such a hit never earns time-windowed credit.
    now_ms: Option<u64>,
}

/// Short-lived current-owner authority for the fixed one-creature death
/// projection. It cannot commit Ability damage, move actors, admit actors or
/// issue continuity.
pub struct CurrentOwnerCombatDeath<'a> {
    carrier: &'a mut ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
}

/// Borrowed, current-owner position capability for one ordinary actor slot.
/// It cannot admit/remove an actor, issue continuity, or commit Ability damage.
pub struct CurrentOwnerMovementPosition<'a> {
    carrier: &'a mut ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovementLocalPosition {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementFacing {
    North,
    East,
    South,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovementPositionContext(PreProductionPositionContext);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovementPositionSnapshot(PositionSnapshot);

/// Sole non-test producer is the world owner's transaction-borrowing relocation proof.
/// The seal is crate-visible only for that audited producer; raw source plan coordinates
/// and client input never implement it.
pub mod relocation_seal {
    pub trait Sealed {}
}
pub trait SpellRelocationProof: relocation_seal::Sealed {
    fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
    );
}

impl MovementPositionSnapshot {
    pub const fn facing(self) -> Option<MovementFacing> {
        self.0.version.facing
    }
    pub const fn position(self) -> MovementLocalPosition {
        let position = self.0.version.position;
        MovementLocalPosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        }
    }

    pub const fn context(self) -> MovementPositionContext {
        MovementPositionContext(self.0.version.context)
    }

    pub const fn revision(self) -> u64 {
        self.0.version.revision
    }

    pub const fn world_id(self) -> WorldId {
        self.0.version.context.world_id
    }
}

impl CurrentOwnerMovementPosition<'_> {
    pub fn commit_spell_relocation<P: SpellRelocationProof>(
        &mut self,
        proof: P,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let (actor, expected, next) = proof.parts();
        if actor.0 != expected.0.actor_ref {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        let snapshot = self.carrier.compare_commit_position(
            self.continuity,
            expected.0,
            expected.0.version.context,
            LocalPosition {
                x: next.x,
                y: next.y,
                floor: next.floor,
            },
        )?;
        Ok(MovementPositionSnapshot(snapshot))
    }
    pub fn read(&self, actor: ExactActorRef) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .read_movement_position(self.continuity, actor.0)
    }

    pub fn commit_cardinal(
        &mut self,
        expected: MovementPositionSnapshot,
        next: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let before = expected.position();
        let east = before.x.checked_add(1).is_some_and(|x| x == next.x) && next.y == before.y;
        let west = before.x.checked_sub(1).is_some_and(|x| x == next.x) && next.y == before.y;
        let south = before.y.checked_add(1).is_some_and(|y| y == next.y) && next.x == before.x;
        let north = before.y.checked_sub(1).is_some_and(|y| y == next.y) && next.x == before.x;
        if next.floor != before.floor || !(east || west || south || north) {
            return Err(CarrierError::MovementNonCardinal);
        }
        self.carrier.commit_movement_position(
            self.continuity,
            expected.0,
            LocalPosition {
                x: next.x,
                y: next.y,
                floor: next.floor,
            },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerDamageResult {
    pub applied: bool,
    pub health_before: i64,
    pub health_after: i64,
}

/// Immutable, borrowed Ability proposal. Authority is checked by the owner,
/// independently of these bytes, before the single slot replacement.
pub struct OwnerDamageCommand<'a> {
    pub target: &'a [u8],
    pub occurrence: &'a [u8],
    pub binding: &'a [u8],
    pub damage: i64,
}

/// `COMBAT01-DAMAGE-RECEIPTS-PER-CREATURE-GENERATION` (`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
/// D140, `docs/architecture/reviews/OTERYN_GAME_D4_MULTI_HIT_DAMAGE_RECEIPT_DECISION_2026-09-29.md`
/// section 4.1): at most this many committed damage receipts are retained at once per live
/// creature actor generation. Retention, not admission: an evictable oldest receipt is evicted to
/// admit a new distinct occurrence (D141); only 16 simultaneously non-evictable receipts refuse
/// one with [`CarrierError::DamageReceiptCapacityExceeded`].
pub const COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX: usize = 16;

/// Mirror of the already-registered `ABILITY01-EFFECT-PLAN-ENTRIES` hard maximum
/// (`ability/mod.rs` `MAX_EFFECT_PLAN_ENTRIES`, no new row): the number of distinct
/// `sub_ordinal` values one command can produce. Foundation must not import Ability, so the
/// value is mirrored here and pinned to it by a test in `channel_owner_ability_commit_tests.rs`.
pub const ABILITY01_EFFECT_PLAN_ENTRIES_MAX: u16 = 2;

/// `(CharacterId, character_lease_generation, GameSessionId, CommandId sequence, sub_ordinal)`:
/// the carrier-derived replay identity of one attributed damage occurrence (D141 point 3).
pub type DamageOrigin = (CharacterId, u64, GameSessionId, u64, u16);

/// `(character_lease_generation, GameSessionId, sequence, sub_ordinal)`: one attacker's
/// high-water mark (D141 point 4). The lease generation is the per-character monotonic
/// generation of the durable `CharacterLease` (`CurrentCharacterGameplayFence::
/// character_lease_generation`), which is what proves at this mutation boundary which session is
/// current: a lower generation is a superseded session.
type HighWater = (u64, GameSessionId, u64, u16);

/// D141: the command identity of one attributed damage occurrence. Built only from a real
/// [`CommandRef`] (never a caller-supplied opaque id) plus the attacking `CharacterId` and the
/// effect's own index within its Effect Plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackerCommand {
    pub character: CharacterId,
    lease_generation: u64,
    pub session: GameSessionId,
    pub sequence: u64,
    pub sub_ordinal: u16,
}

impl AttackerCommand {
    /// `lease_generation` is the attacker's current `character_lease_generation` (non-zero).
    pub const fn new(
        character: CharacterId,
        lease_generation: u64,
        command: CommandRef,
        sub_ordinal: u16,
    ) -> Self {
        Self {
            character,
            lease_generation,
            session: command.game_session_id(),
            sequence: command.command_id().get(),
            sub_ordinal,
        }
    }

    pub const fn origin(self) -> DamageOrigin {
        (
            self.character,
            self.lease_generation,
            self.session,
            self.sequence,
            self.sub_ordinal,
        )
    }

    const fn high_water(self) -> HighWater {
        (
            self.lease_generation,
            self.session,
            self.sequence,
            self.sub_ordinal,
        )
    }
}

/// One retained damage receipt. `origin` and `ordinal` are immutable once set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerCommitRecord {
    pub binding: Box<[u8]>,
    damage: i64,
    pub result: OwnerDamageResult,
    /// D142: this creature generation's owner damage-application ordinal of this receipt.
    pub ordinal: u64,
    /// D141: `Some` for an attributed commit (its replay identity), `None` for an unsequenced
    /// one, which is never evictable.
    pub origin: Option<DamageOrigin>,
}

/// D140/D142: the bounded, ephemeral, per-creature-generation receipt list and the owner
/// damage-application ordinal counter. Fresh on every admission, dropped with the slot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DamageReceipts {
    pub entries: Vec<OwnerCommitRecord>,
    pub next_ordinal: u64,
    /// The owner clock (ms) of the turn that committed the lethal receipt, when that turn
    /// supplied one. Set once by the lethal commit; a replay never changes it.
    lethal_at_ms: Option<u64>,
}

impl DamageReceipts {
    /// D144: the one retained receipt whose commit drove health to zero, if any.
    pub fn lethal(&self) -> Option<&OwnerCommitRecord> {
        self.entries.iter().find(|record| {
            record.result.applied
                && record.result.health_before > 0
                && record.result.health_after == 0
        })
    }

    /// D141 point 4: the oldest evictable receipt. A receipt is evictable iff it is sequenced,
    /// its attacker is currently tracked with a high-water mark, and either its origin lease
    /// generation was superseded (fenced, and refused by [`DamageContributors::admission`], so
    /// it can never be re-applied) or the current session's mark covers its
    /// `(sequence, sub_ordinal)`. An unsequenced receipt is never evictable.
    fn oldest_evictable(&self, contributors: &DamageContributors) -> Option<usize> {
        self.entries.iter().position(|record| {
            let Some((character, lease, session, sequence, sub_ordinal)) = record.origin else {
                return false;
            };
            let Some((mark_lease, mark_session, mark_sequence, mark_sub)) =
                contributors.high_water(character)
            else {
                return false;
            };
            mark_lease > lease
                || (mark_lease == lease
                    && mark_session == session
                    && (mark_sequence, mark_sub) >= (sequence, sub_ordinal))
        })
    }
}

/// `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` (`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
/// D132, `docs/architecture/reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md`
/// §4.3): at most this many distinct `CharacterId` contributors are tracked per live creature
/// actor. Past this bound, a creature's own damage/HP math stays completely unaffected -- only a
/// *new* (17th+) distinct attacker's damage stops being added to the map; every already-tracked
/// contributor keeps accumulating normally (fail-open for combat, fail-closed only for
/// attribution).
pub const COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX: usize = 16;

/// D132: one tracked attacker's running damage total against a live creature actor, and the
/// ordinal (this creature's own damage-application sequence) at which that total was last
/// reached. A contributor's total only ever increases, so "last reached" already is "first
/// reached this exact value" -- no separate history is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageContributor {
    character: CharacterId,
    pub total: u64,
    last_update_ordinal: u64,
    /// D141: this attacker's `(session, sequence, sub_ordinal)` high-water mark. A new current
    /// session replaces it outright; within one session it only ever rises.
    high_water: Option<HighWater>,
    /// The latest owner clock (ms) at which this attacker's damage was recorded. A hit whose
    /// turn supplied no clock leaves it unchanged, so it never moves later than a known hit.
    last_damage_at_ms: Option<u64>,
}

/// D132/D3-3: bounded, ephemeral, per-creature-generation damage-contributor accumulation.
/// Lives only inside its owning `Slot::CreatureOccupied` -- a fresh, empty map on every creature
/// admission (`admit_inner`), dropped with the slot on `remove` (administrative removal or
/// respawn). Never grows past `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX` and never
/// initialized from, or retained across, a prior generation: a scope move drops whatever
/// in-progress accumulation was pending, exactly the accepted D52 loss ("Przepadają, bez
/// duplikatów" -- nothing duplicated, nothing silently completed by a later generation).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DamageContributors {
    pub entries: Vec<DamageContributor>,
}

impl DamageContributors {
    /// Attribute one already-applied, positive damage amount to `character`. A *new* distinct
    /// contributor past `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX` is silently not tracked:
    /// the caller's own HP mutation already committed independently of this call and is never
    /// affected by it. An already-tracked contributor keeps accumulating regardless of map size.
    ///
    /// `ordinal` is the owner damage-application ordinal (D142) the carrier assigned to this
    /// commit at the same mutation boundary; this accumulator keeps no counter of its own.
    /// `high_water`, when supplied, replaces the attacker's mark (D141). `at_ms` is the
    /// committing owner turn's clock.
    fn record(
        &mut self,
        character: CharacterId,
        damage: u64,
        ordinal: u64,
        high_water: Option<HighWater>,
        at_ms: Option<u64>,
    ) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|contributor| contributor.character == character)
        {
            existing.total = existing.total.saturating_add(damage);
            existing.last_update_ordinal = ordinal;
            if high_water.is_some() {
                existing.high_water = high_water;
            }
            existing.last_damage_at_ms = existing.last_damage_at_ms.max(at_ms);
            return;
        }
        if self.entries.len() >= COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX {
            return;
        }
        self.entries.push(DamageContributor {
            character,
            total: damage,
            last_update_ordinal: ordinal,
            high_water,
            last_damage_at_ms: at_ms,
        });
    }

    /// D141: `character`'s current high-water mark, if it is tracked and has one.
    fn high_water(&self, character: CharacterId) -> Option<HighWater> {
        self.entries
            .iter()
            .find(|contributor| contributor.character == character)
            .and_then(|contributor| contributor.high_water)
    }

    /// D141 admission of a new attributed occurrence against its attacker's mark, ordered by
    /// the per-character `character_lease_generation` (which proves which session is current):
    /// a lower generation, or an equal one with a differing `GameSessionId`, is a superseded
    /// session (`SupersededAttackerSession`); an equal generation and session at or below the
    /// mark's `(sequence, sub_ordinal)` is `StaleAttackerSequence`; a higher generation is a
    /// genuinely newer session (FND-04B sections 16 and 21) and replaces the mark.
    fn admission(&self, attacker: AttackerCommand) -> Result<(), CarrierError> {
        let Some((lease, session, sequence, sub_ordinal)) = self.high_water(attacker.character)
        else {
            return Ok(());
        };
        match attacker.lease_generation.cmp(&lease) {
            std::cmp::Ordering::Less => Err(CarrierError::SupersededAttackerSession),
            std::cmp::Ordering::Greater => Ok(()),
            std::cmp::Ordering::Equal if attacker.session != session => {
                Err(CarrierError::SupersededAttackerSession)
            }
            std::cmp::Ordering::Equal
                if (attacker.sequence, attacker.sub_ordinal) <= (sequence, sub_ordinal) =>
            {
                Err(CarrierError::StaleAttackerSequence)
            }
            std::cmp::Ordering::Equal => Ok(()),
        }
    }

    /// D132's deterministic two-rule tie-break: the highest running total wins; among equal top
    /// totals, the contributor whose total *first reached* that value (the smaller
    /// `last_update_ordinal`) wins; a remaining same-ordinal tie resolves to the
    /// lexicographically lowest `CharacterId` byte sequence (`CharacterId`'s derived `Ord` is
    /// exactly that byte order, `crates/protocol-oteryn/src/lib.rs`'s
    /// `foundation_uuid_v7_id!`). A pure function of already-recorded state, independent of
    /// insertion or iteration order.
    fn top_damage_character(&self) -> Option<CharacterId> {
        self.top_contributor()
            .map(|contributor| contributor.character)
    }

    /// The whole winning entry under the same tie-break as [`Self::top_damage_character`].
    fn top_contributor(&self) -> Option<DamageContributor> {
        self.entries.iter().copied().reduce(|best, candidate| {
            if is_more_preferred_contributor(&candidate, &best) {
                candidate
            } else {
                best
            }
        })
    }
}

/// `true` when `candidate` outranks `current_best` under D132's tie-break order (see
/// [`DamageContributors::top_damage_character`]).
fn is_more_preferred_contributor(
    candidate: &DamageContributor,
    current_best: &DamageContributor,
) -> bool {
    match candidate.total.cmp(&current_best.total) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => {
            match candidate
                .last_update_ordinal
                .cmp(&current_best.last_update_ordinal)
            {
                std::cmp::Ordering::Less => true,
                std::cmp::Ordering::Greater => false,
                std::cmp::Ordering::Equal => candidate.character < current_best.character,
            }
        }
    }
}

/// §1.3: the reward principal identity of a creature's top-damage contributor, read in the
/// owner turn that projects its death.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopDamageContributor {
    pub character: CharacterId,
    /// The lease generation and session of the winner's D141 high-water mark.
    pub lease_generation: u64,
    pub session: GameSessionId,
    /// The winner's committed player slot in this carrier, bound to that same session and lease.
    pub actor: ExactActorRef,
    pub last_damage_at_ms: Option<u64>,
}

/// §1.3: who the top-damage reward principal of a projected death is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopDamagePrincipal {
    /// No tracked contributor: the lethal attacker is the principal.
    Untracked,
    /// The winner's slot is gone, or it now holds another session or lease generation.
    Gone,
    Present(TopDamageContributor),
}

impl TopDamagePrincipal {
    /// The tracked winner, if any; callers outside Foundation cannot name this enum.
    pub const fn present(self) -> Option<TopDamageContributor> {
        match self {
            Self::Present(winner) => Some(winner),
            Self::Untracked | Self::Gone => None,
        }
    }

    pub const fn is_gone(self) -> bool {
        matches!(self, Self::Gone)
    }
}

/// Stable identity of the one committed lethal occurrence. Construction stays
/// private to the physical Channel owner; callers cannot supply occurrence
/// bytes, HP facts or actor generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureDeathOccurrenceRef {
    actor: ExactActorRef,
    pub commit_binding: Box<[u8]>,
    damage: i64,
    health_before: i64,
}

impl CreatureDeathOccurrenceRef {
    pub const fn actor(&self) -> ExactActorRef {
        self.actor
    }

    pub fn commit_binding(&self) -> &[u8] {
        &self.commit_binding
    }

    pub const fn damage(&self) -> i64 {
        self.damage
    }

    pub const fn health_before(&self) -> i64 {
        self.health_before
    }

    /// The durable, restart-stable death key of this committed lethal
    /// occurrence (`CREATURE-DEATH-OCCURRENCE-IDENTITY-V1`, DUR-03 decision
    /// §4.1): exactly the owner-issued actor reference. The commit binding,
    /// damage and HP facts stay bound attributes, never key material.
    pub const fn death_key(&self) -> CreatureDeathOccurrenceKey {
        CreatureDeathOccurrenceKey(self.actor.0)
    }
}

/// `(WorldId, ChannelId, ScopeOwnershipGeneration, ActorLocalId,
/// ActorLocalGeneration)` of one committed creature death. The only
/// constructor is [`CreatureDeathOccurrenceRef::death_key`]: there is no
/// raw-byte, field or caller constructor, so a key always names a lethal
/// occurrence the physical Channel owner committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureDeathOccurrenceKey(ActorRef);

impl CreatureDeathOccurrenceKey {
    pub const fn world_id(self) -> WorldId {
        self.0.world_id
    }

    pub const fn channel_id(self) -> ChannelId {
        self.0.channel_id
    }

    pub const fn scope_ownership_generation(self) -> ScopeOwnershipGeneration {
        self.0.scope_generation
    }

    pub const fn actor_local_id(self) -> u32 {
        self.0.actor_local_id.0
    }

    pub const fn actor_local_generation(self) -> u64 {
        self.0.actor_local_generation.0
    }
}

/// One runtime-owned, non-persistent corpse projection. The position is the
/// immutable current-owner position captured by the same slot state that holds
/// the lethal commit; dead creatures cannot mutate or initialize it later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCorpseProjection {
    pub occurrence: CreatureDeathOccurrenceRef,
    pub position: VersionedPosition,
    /// The lethal receipt's owner clock (ms), when its committing turn supplied one.
    death_at_ms: Option<u64>,
}

impl RuntimeCorpseProjection {
    pub const fn occurrence(&self) -> &CreatureDeathOccurrenceRef {
        &self.occurrence
    }

    pub const fn position(&self) -> MovementLocalPosition {
        MovementLocalPosition {
            x: self.position.position.x,
            y: self.position.position.y,
            floor: self.position.position.floor,
        }
    }

    pub const fn position_revision(&self) -> u64 {
        self.position.revision
    }

    pub const fn context_markers(&self) -> (u64, u64, u64) {
        (
            self.position.context.coordinate_frame_marker,
            self.position.context.map_revision_marker,
            self.position.context.content_generation_marker,
        )
    }
}

/// VIS-3: one actor of [`ChannelRuntimeV1::visible_entities`]; `revision` is the position
/// revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleRuntimeActor {
    pub actor: ExactActorRef,
    pub generation: u64,
    pub position: MovementLocalPosition,
    pub revision: u64,
}

/// VIS-3: the entities of [`ChannelRuntimeV1::visible_entities`], by kind.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VisibleRuntimeEntities {
    pub players: Vec<VisibleRuntimeActor>,
    pub creatures: Vec<VisibleRuntimeActor>,
    /// NPC-RT-1: the placed NPC actors (`EntityKind::Npc`).
    pub npcs: Vec<VisibleRuntimeActor>,
}

/// Opaque, single-use handoff from the owner commit record to Combat. It is
/// deliberately neither Clone nor Copy and has no caller-visible constructor.
#[derive(Debug, PartialEq, Eq)]
pub struct CommittedLethalReceipt {
    pub projection: RuntimeCorpseProjection,
}

/// 4096 is the registered Ability plan bound; the owner still independently
/// checks the actual complete encoding before allocating its one receipt.
pub const MAX_OWNER_COMMIT_BINDING_BYTES: usize = 4_096;

impl CurrentOwnerExactActorCommit<'_> {
    pub fn commit_damage(
        &mut self,
        actor: ExactActorRef,
        command: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.carrier.commit_creature_damage_with_pin_inner(
            self.continuity,
            actor.0,
            command,
            None,
            false,
            self.content,
            Some(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
            self.now_ms,
        )
    }

    /// D132/D3-3 + D141: the same owner-authoritative damage commit, additionally attributing the
    /// applied HP loss to `attacker`'s running per-creature damage total
    /// (`COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE`, capped at
    /// [`COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX`]). The replay identity of an attributed
    /// commit is derived by the carrier from `attacker` alone: `command.occurrence` is not read,
    /// so a caller can never present one command's identity under another's bytes. An idempotent
    /// replay never double-attributes: it never reaches the mutation boundary a second time.
    /// Test only: the attacker is supplied, not read from a bound slot. Production writes go
    /// through [`Self::commit_damage_for_bound_attacker`] (A2, D295 item 4).
    #[cfg(any(test, feature = "test-support"))]
    pub fn commit_damage_for_attacker(
        &mut self,
        actor: ExactActorRef,
        attacker: AttackerCommand,
        command: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.carrier.commit_creature_damage_with_pin_inner(
            self.continuity,
            actor.0,
            command,
            Some(attacker),
            false,
            self.content,
            Some(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
            self.now_ms,
        )
    }
}

impl CurrentOwnerExactActorCommit<'_> {
    /// A2 (D295 item 4): the current lease of the attacker's bound player slot. The slot must hold
    /// the command's session, carry no write fence and have a bound lease, checked in that order
    /// (CHARM-DESC-FENCE-LEASE §3 item 2); anything else is `SupersededAttackerSession`.
    pub fn bound_attacker_lease(
        &self,
        attacker: ExactActorRef,
        command: CommandRef,
    ) -> Result<CharacterLease, CarrierError> {
        self.carrier
            .bound_attacker_lease(self.continuity, attacker.0, command.game_session_id())
    }

    /// The attributed damage commit whose attacker authority is the bound slot of `attacker`,
    /// read in the same runtime-lock critical section as the write.
    pub fn commit_damage_for_bound_attacker(
        &mut self,
        actor: ExactActorRef,
        attacker: ExactActorRef,
        command: CommandRef,
        sub_ordinal: u16,
        damage: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let lease = self.bound_attacker_lease(attacker, command)?;
        self.carrier.commit_creature_damage_with_pin_inner(
            self.continuity,
            actor.0,
            damage,
            Some(AttackerCommand::new(
                lease.character_id(),
                lease.generation(),
                command,
                sub_ordinal,
            )),
            false,
            self.content,
            Some(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
            self.now_ms,
        )
    }

    /// ATTACK-1b swing identity: every swing of one attack lineage reuses the lineage's
    /// `CommandRef` (the `AttackTarget` command), and `swing_ordinal` is the swing's index within
    /// that lineage. The bound-attacker session fence is unchanged. Within one lineage the
    /// ordinal is strictly increasing (a replay returns the retained receipt; a lower or equal
    /// unretained ordinal is `StaleAttackerSequence`); against other commands of the same session
    /// the admission is deferred like a SPELL-BATCH completion, so the high-water mark never
    /// lowers.
    pub fn commit_swing_damage_for_bound_attacker(
        &mut self,
        actor: ExactActorRef,
        attacker: ExactActorRef,
        lineage: CommandRef,
        swing_ordinal: u16,
        damage: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let lease = self.bound_attacker_lease(attacker, lineage)?;
        let command = AttackerCommand::new(
            lease.character_id(),
            lease.generation(),
            lineage,
            swing_ordinal,
        );
        self.carrier
            .swing_lineage_admission(self.continuity, actor.0, command)?;
        // Every u16 swing ordinal is admissible: the lineage's last ordinal (`u16::MAX`) still
        // swings, and attack.rs ends the lineage after it.
        self.carrier.commit_creature_damage_inner_limited(
            self.continuity,
            actor.0,
            damage,
            Some(command),
            false,
            None,
            true,
            self.now_ms,
        )
    }
}

impl CurrentOwnerCombatDeath<'_> {
    pub fn committed_lethal_receipt(
        &self,
        actor: ExactActorRef,
    ) -> Result<CommittedLethalReceipt, CarrierError> {
        self.carrier
            .committed_lethal_receipt_inner(self.continuity, actor.0)
    }

    pub fn project_committed_lethal(
        &mut self,
        receipt: CommittedLethalReceipt,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        self.carrier
            .project_committed_lethal_inner(self.continuity, receipt, false, false)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn project_committed_lethal_with_failures(
        &mut self,
        receipt: CommittedLethalReceipt,
        fail_before_write: bool,
        fail_after_write: bool,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        self.carrier.project_committed_lethal_inner(
            self.continuity,
            receipt,
            fail_before_write,
            fail_after_write,
        )
    }

    /// D2b: the death key and corpse position of this generation's projected
    /// committed death of `actor`, read from the owner's own projection.
    pub fn projected_death(
        &self,
        actor: ExactActorRef,
    ) -> Result<(CreatureDeathOccurrenceKey, MovementLocalPosition), CarrierError> {
        self.carrier.validate_ref(self.continuity, actor.0)?;
        if self.carrier.is_native_summon(actor) {
            return Err(CarrierError::SummonHasNoRewards);
        }
        self.carrier
            .corpse_projections
            .iter()
            .find(|projection| projection.occurrence.actor == actor)
            .map(|projection| (projection.occurrence.death_key(), projection.position()))
            .ok_or(CarrierError::CommittedLethalUnavailable)
    }

    /// P2 4180039074: the owner clock of the turn that committed `actor`'s lethal receipt, read
    /// from the retained projection, so a replayed projection keeps its original death time.
    /// `Ok(None)` when that turn supplied no clock.
    pub fn projected_death_at_ms(&self, actor: ExactActorRef) -> Result<Option<u64>, CarrierError> {
        self.carrier.validate_ref(self.continuity, actor.0)?;
        self.carrier
            .corpse_projections
            .iter()
            .find(|projection| projection.occurrence.actor == actor)
            .map(|projection| projection.death_at_ms)
            .ok_or(CarrierError::CommittedLethalUnavailable)
    }

    /// §1.3: the top-damage reward principal of `actor`'s still-live slot, under the D132
    /// tie-break of [`Self::top_damage_character`], with the winner's session identity, slot and
    /// last damage time. See [`ChannelActorCarrier::top_damage_contributor_inner`].
    pub fn top_damage_contributor(
        &self,
        actor: ExactActorRef,
    ) -> Result<TopDamagePrincipal, CarrierError> {
        self.carrier
            .top_damage_contributor_inner(self.continuity, actor.0)
    }

    /// D2b: the memoized XP `ExperienceRewardOccurrence` bytes of this
    /// generation's committed death for `character`, minting them on first
    /// call. See [`ChannelActorCarrier::reward_occurrence_inner`].
    pub fn reward_occurrence(
        &mut self,
        actor: ExactActorRef,
        character: [u8; 16],
    ) -> Result<([u8; 16], bool), CarrierError> {
        self.carrier
            .reward_occurrence_inner(self.continuity, actor.0, character)
    }

    /// D132/D3-3: the deterministic top-damage `CharacterId` for `actor`'s still-live slot in
    /// this generation, read from the owner's own bounded, in-memory `DamageContributors`
    /// accumulation. `Ok(None)` when the creature died (or is still alive) with no tracked
    /// contributor ever recorded against it (a damage-free death, or every hit came from an
    /// untracked 17th-plus attacker). This reads only live slot state, never
    /// `corpse_projections`: like the rest of D132's accumulation, the answer does not outlive
    /// this generation (D52). D3-2 is the composition point that reads this at the same moment
    /// it extracts `(death, corpse)`, before the corpse's own MINT.
    #[allow(
        dead_code,
        reason = "no production caller yet; D3-2 wires this into settle_creature_death_rewards"
    )]
    pub fn top_damage_character(
        &self,
        actor: ExactActorRef,
    ) -> Result<Option<CharacterId>, CarrierError> {
        self.carrier
            .top_damage_character_inner(self.continuity, actor.0)
    }
}

impl CurrentOwnerExactActorLookup<'_> {
    /// One direct slot/generation lookup. Invalid, vacant and misrouted refs
    /// share one failure; no actor payload or carrier authority is returned.
    pub fn contains(&self, actor: ExactActorRef) -> bool {
        let Ok(index) = self.carrier.validate_ref(self.continuity, actor.0) else {
            return false;
        };
        match &self.carrier.slots[index] {
            Slot::Occupied {
                generation,
                committed,
                ..
            } => *committed && *generation == actor.0.actor_local_generation.0,
            Slot::CreatureOccupied {
                generation, health, ..
            } => *generation == actor.0.actor_local_generation.0 && *health > 0,
            Slot::VacantReusable { .. }
            | Slot::Exhausted { .. }
            | Slot::CreatureReserved { .. }
            | Slot::NpcOccupied { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorState(pub u64);

/// The binding a position is valid under. In production the Channel's fixed
/// Content pin (#935) holds the full binding once; each slot keeps only this
/// compact per-runtime reference to it (the measured 192-byte slot footprint
/// is unchanged): the frame-binding and map-revision digest prefixes and the
/// activation sequence. Test fixtures use synthetic values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreProductionPositionContext {
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub scope_generation: ScopeOwnershipGeneration,
    pub coordinate_frame_marker: u64,
    pub map_revision_marker: u64,
    pub content_generation_marker: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalPosition {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionedPosition {
    actor_local_id: ActorLocalId,
    actor_local_generation: ActorLocalGeneration,
    pub context: PreProductionPositionContext,
    pub position: LocalPosition,
    pub revision: u64,
    facing: Option<MovementFacing>,
}

/// A value snapshot, never an authority token. Compare-commit revalidates
/// actor, independently current owner, context and revision at the write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionSnapshot {
    pub actor_ref: ActorRef,
    pub version: VersionedPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Slot {
    VacantReusable {
        generation: u64,
        next_free: Option<u32>,
    },
    Occupied {
        generation: u64,
        actor: ActorState,
        /// Present only for the production player binding; fixture actors use None.
        game_session_id: Option<GameSessionId>,
        /// False while capacity is reserved but durable fresh admission is unresolved.
        committed: bool,
        position: Option<VersionedPosition>,
        /// Present while the committed player's durable GameSession is RECONNECTABLE
        /// after an authoritative control loss (`DISCONNECT-PROTECTION-V1`).
        lifecycle: Box<PlayerRuntimeState>,
        spell_combat: Box<runtime_actor_spell::ActorCombatState>,
    },
    CreatureOccupied {
        generation: u64,
        actor: ActorState,
        position: Option<VersionedPosition>,
        target_identity: Arc<[u8]>,
        health: i64,
        /// D140: the bounded receipt list and owner damage-application ordinal (D142). Boxed for
        /// the same footprint reason as `damage_contributors`.
        committed: Box<CreatureCommitState>,
        /// D132/D3-3: this generation's running per-attacker damage accumulation. Fresh and
        /// empty on every admission; dropped with the slot on `remove`. Boxed for the same
        /// reason `target_identity` is `Arc<[u8]>` rather than inline bytes: this variant's own
        /// extra state must not grow every `Slot` (including every non-creature player slot) by
        /// more than a pointer's worth of the already-measured fixed-slot footprint.
        damage_contributors: Box<DamageContributors>,
        companion: Option<Box<runtime_actor_companion::CompanionState>>,
        spell_combat: Box<runtime_actor_spell::ActorCombatState>,
    },
    /// Real fixed capacity retained while a durable familiar COMMIT is unresolved.
    /// The complete physical payload is allocated before SQL and moved on receipt.
    CreatureReserved {
        generation: u64,
        owner: ExactActorRef,
        session: GameSessionId,
        planned: Box<Slot>,
    },
    Exhausted {
        generation: u64,
    },
    /// NPC-RT-1 (NPC-BEHAVIOUR-0 §3.1): one runtime-only NPC actor of one placement. Never a
    /// combat creature: no health, target identity or spell state; it blocks movement and is
    /// shown, and nothing removes it within the generation.
    NpcOccupied {
        generation: u64,
        key: Arc<str>,
        position: VersionedPosition,
    },
}

/// The durable loss decision mirrored by the Channel owner for its present, uncontrolled
/// player actor. Epoch and grace deadline are the committed `ControlLossEpoch` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlLossMark {
    pub epoch: u64,
    pub grace_deadline: i64,
}

/// CHARM-DESC-FENCE-LEASE §3 item 2: the token of one transition's write fence on a player slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteFenceToken {
    /// Grace expiry: the control-loss epoch the slot still carries.
    ControlLoss(u64),
    /// Any other transition that ends the session's hold: an id minted by
    /// [`ChannelRuntimeV1::mint_transition_fence`] for the caller.
    Transition(u64),
}

/// How [`ChannelRuntimeV1::fence_player_writes`] set the fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteFenceSet {
    /// The fence is new.
    Fenced,
    /// A retry of the same transition: the existing fence carries the same token.
    Joined,
}

/// The durable reconcile of a same-session continuation after process replacement
/// (CHARM-DESC-FENCE-LEASE §3 item 7, FND-04B §22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "process-replacement continuation is not composed yet; the carrier gate is"
)]
pub enum ContinuationReconcile {
    /// The durable row is terminal: no same-session continuation.
    Terminal,
    /// Non-terminal and the §22 evidence is proven, under this lease.
    Proven(CharacterLease),
    /// The binding cannot be proven: fail closed.
    Unprovable,
}

/// A2: the attacker authority of one committed player slot, kept beside the slot so the fixed
/// slot footprint is unchanged.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PlayerAttackerAuthority {
    /// The admitted session's Character lease. Absent means no damage write is admitted.
    lease: Option<CharacterLease>,
    /// Set while a transition that ends the session's hold is unsettled.
    fence: Option<WriteFenceToken>,
}

/// Channel-owner facts about one committed, present player actor of an exact GameSession.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerControlFacts {
    /// Stable opaque identity of this actor's placement in this runtime scope.
    pub placement_identity: [u8; 16],
    /// The actor's local generation: non-zero and unchanged while it stays placed.
    pub placement_revision: u64,
    pub control_loss: Option<ControlLossMark>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelActorCarrier {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_generation: ScopeOwnershipGeneration,
    pub slots: Box<[Slot]>,
    free_head: Option<u32>,
    /// AI-2 (GAME-AI-01 §4.1: "the carrier's current one-creature limit ...
    /// is lifted to this envelope in AI-2"): one committed death per creature
    /// generation, keyed by that creature's own `ExactActorRef`. Bounded by
    /// `slots.len()` itself -- no more than one dead-but-not-yet-removed
    /// creature can exist per slot, so this can never exceed total capacity.
    /// `remove` prunes an actor's entry so it never outlives its slot.
    pub corpse_projections: Vec<RuntimeCorpseProjection>,
    /// D2b: the memoized XP reward occurrence of each committed death, minted
    /// at most once per actor (DUR-03 decision §4.2). AI-2 widens this from
    /// its original one-creature-per-generation singleton to one entry per
    /// dead creature actor, same bound and pruning as `corpse_projections`. A
    /// fresh carrier is bootstrapped per ownership generation, so this is
    /// never initialized from a prior generation: a scope move drops whatever
    /// was pending here rather than reusing or duplicating it (D52).
    death_reward_occurrences: Vec<DeathRewardOccurrence>,
    /// AI-2 (§4.3, D116): realized spawn sources and their per-cell live/
    /// pending/retry state for this carrier's ownership generation. Kept in strictly
    /// ascending source order, so a source is found by binary search (`spawn_index`) and
    /// the iteration order is the canonical (source, point ordinal) order.
    spawns: Vec<SpawnRealization>,
    native_summons: NativeSummonLinks,
    native_summon_deaths: [Option<NativeSummonDeath>; 64],
    native_summon_death_cursor: usize,
    bone_shared_groups: Vec<NativeBoneSharedGroup>,
    /// The last write-fence transition id this carrier minted (CHARM-DESC-FENCE-LEASE §3 item 2).
    fence_transitions: u64,
    /// A2: the bound lease and write fence of each committed player slot that has one, keyed by
    /// slot index and that slot's generation. At most one entry per slot: `remove` prunes it, so
    /// it is bounded by `slots.len()` and never outlives its actor.
    pub attackers: Vec<PlayerAttackerEntry>,
    /// VIS-3 (Codex 4178196302): the index of every slot off the free list (`Occupied`,
    /// `CreatureOccupied` or a companion's `CreatureReserved`), in no particular order, so a
    /// census of the actors present costs their number, not the capacity. Its capacity is
    /// reserved at bootstrap for every slot, so taking a slot never reallocates it; `remove`, the
    /// companion rollback and the spawn rollbacks keep it equal to those slots (Codex 4178855592).
    occupied: Vec<u32>,
    /// MAP-ITEM-REF-1 Part B: the corpse Item bound to each retained corpse projection, at most
    /// one per projection; [`ChannelRuntimeV1::bind_corpse_item`] drops the evicted ones.
    corpse_item_bindings: Vec<CorpseItemBinding>,
    /// Codex 4248192466 (LOOT-LIVE-1): the killed actors whose retained corpse projection an
    /// accepted, not yet final kill settlement still binds to. A pinned projection is never
    /// evicted ([`ChannelRuntimeV1::pin_corpse_projection`]); at most
    /// `MAX_PINNED_CORPSE_PROJECTIONS`, each naming a retained projection.
    corpse_projection_pins: Vec<ExactActorRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerAttackerEntry {
    index: usize,
    generation: u64,
    authority: PlayerAttackerAuthority,
}

/// D2b: one memoized `(reward principal, ExperienceRewardOccurrence)` pair
/// for this generation's committed death. Not durable: it lives only in the
/// physical Channel owner's in-process state, matching the accepted D52
/// decision that no durable death row is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeathRewardOccurrence {
    actor: ActorRef,
    character: [u8; 16],
    occurrence: [u8; 16],
}

// AI-2 (GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1 §4.3, §4.9, D115/D116) and
// SPAWN-1a (CREATURE-AI-0 §6.2, §6.3): spawn realization and respawn.
// `RESOURCE_LIMITS_REGISTRY.json` is the source of truth for these ceilings; each constant
// below equals the registered value of the row it names.

/// `CREATUREAI0-RL-02`: spawn sources realized in one Channel (CREATURE-AI-0 §10; it replaces
/// `AI01-SPAWN-SOURCES-PER-SCOPE` 16). Source ordinals `0..65,536` fit [`SpawnSourceId`].
pub const CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX: usize = 65_536;
/// `CREATUREAI0-RL-03`: points of one spawn source, one creature and one declared cell each
/// (it replaces `AI01-SPAWN-POPULATION` 4 and `AI01-SPAWN-PLACEMENT-CELLS` 4). Point ordinals
/// `0..64` fit [`spawn::SpawnPoint`].
pub const CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX: usize = 64;
/// `CREATUREAI0-RL-01`: spawn points, and so spawned creatures, in one Channel. This, not the
/// product of the two rows above, bounds the carrier's spawn state and spawned population; it
/// equals `CREATUREAI0-RL-12` (one pending respawn occurrence per point).
pub const CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX: usize = 131_072;
/// `CREATUREAI0-RL-04`: creature actors of every kind in one Channel (CREATURE-AI-0 §10; it
/// replaces `AI01-ACTIVE-ACTORS` 256). [`ChannelActorCarrier::admit_creature`], the one creature
/// admission, refuses `CapacityExceeded` once this many slots are taken. A creature takes a slot,
/// so this counts every actor: at the pre-production capacity of 131,072 slots it never refuses,
/// and a larger carrier refuses a creature at 262,144 actors of any kind, never later. The
/// creature bite ledger (`CREATURE_BITE_LEDGER_MAX`, held equal by a test) and the melee owner,
/// one entry per slot, are bounded by the same value.
pub const CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX: usize = 262_144;
/// `CREATUREAI0-RL-08`: monster summon entries per creature (CREATURE-AI-0 §10; content
/// validation refuses more than 8 entries and 16 summons per creature).
pub const CREATUREAI0_SUMMON_ENTRIES_PER_CREATURE_MAX: usize = 8;
/// `CREATUREAI0-RL-07`: defence Ability proposals per creature (8 of the 24 per think).
pub const CREATUREAI0_DEFENCES_PER_CREATURE_MAX: usize = 8;
/// SPAWN-CEILINGS-1 (Codex 4242954089, 4243039220): the replay memos of the native summon owner,
/// one per summoner and summon entry, in an [`ActorMemoTable`]. A creature's memos go when its
/// slot's next occupant first touches the table, or when the table is full, so the table holds at
/// most `CREATUREAI0-RL-04` creatures times `CREATUREAI0-RL-08` entries; it replaces the D57
/// envelope of 64 creatures times 8.
pub const NATIVE_SUMMON_MEMOS_PER_CHANNEL_MAX: usize =
    CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX * CREATUREAI0_SUMMON_ENTRIES_PER_CREATURE_MAX;
/// SPAWN-CEILINGS-1 (Codex 4242954089, 4243039220): the replay memos keyed by a creature and one
/// of its defence entries (the defence summon owner and the creature self-heal ledger, each an
/// [`ActorMemoTable`]), at most `CREATUREAI0-RL-04` creatures times `CREATUREAI0-RL-07` defences;
/// it replaces 64 times 8.
pub const CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX: usize =
    CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX * CREATUREAI0_DEFENCES_PER_CREATURE_MAX;
/// `SPAWNCEIL-RL-02` (SPAWN-CEILINGS-1, Codex 4243039217): native summon links one call of
/// [`ChannelRuntimeV1::retire_native_summons`] examines, and so the dead children it retires. The
/// pass resumes where the previous call stopped and wraps, so every link is examined within
/// `ceil(links / 256)` calls; a summon admission first retires its own parent's dead children (at
/// most 16), so a deferred retirement never refuses a summon or lets a parent hold more than 16
/// links.
pub const NATIVE_SUMMON_RETIREMENT_LINKS_PER_CALL_MAX: usize = 256;
/// SPAWN-CEILINGS-1 (Codex 4242245064): carrier slots an activation leaves free for player
/// admission, the D128 design target of 500 concurrent players per Channel. The pre-production
/// carrier holds 131,072 actors (`PREPRODUCTION_FIRST_SLICE_ACTOR_CAPACITY`), as many as
/// `CREATUREAI0-RL-01` spawn points, so an activation that fills it would refuse every player.
/// A carrier smaller than twice this (tests, qualification) keeps half of its slots instead.
pub const SPAWN_ACTIVATION_PLAYER_HEADROOM: usize = 500;

/// The slots an activation must leave free in a carrier of `capacity` slots.
const fn spawn_activation_player_headroom(capacity: usize) -> usize {
    if capacity / 2 < SPAWN_ACTIVATION_PLAYER_HEADROOM {
        capacity / 2
    } else {
        SPAWN_ACTIVATION_PLAYER_HEADROOM
    }
}

/// SPAWN-CEILINGS-1 (Codex 4242270322): respawn steps (a warning or blocked check, an
/// admission or an Occupied check) one Channel owner window runs, however many occurrences
/// fall due together. A step reads at most one pass over the slots (the cell occupancy check)
/// and one over the occupied index (the dead creature's removal, or the present players once
/// per drive), so this bounds the drive's work per window; the occurrences left over stay
/// pending and run first in the next window, by deadline then point.
pub const SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX: usize = 32;

/// SPAWN-CEILINGS-1 (Codex 4243397472): realized points one owner window's gone scan reads,
/// from a cursor kept across windows (canonical order, wrapping), at the activation's
/// `CREATUREAI0-RL-14` rate. A Channel of at most this many points is read whole every window,
/// as before; a larger one is read once per `ceil(points / 4,096)` windows, so a death is
/// observed at most 32 windows (1.6 s) later at the 131,072-point ceiling.
pub const SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX: usize = spawn::REALIZATIONS_PER_WINDOW_MAX;

/// `AI01-SPAWN-SOURCES-PER-SCOPE` and `AI01-SPAWN-POPULATION` (AI-RL-11, D57): the first
/// slice's 16 sources × 4 creatures = 64-creature envelope. Spawn realization no longer reads
/// them (CREATURE-AI-0 §10 replaced them by the `CREATUREAI0_SPAWN_*` rows above), and neither
/// does the production creature path (SPAWN-CEILINGS-1, Codex 4242381779: the creature bite
/// ledger and the melee owner are bounded by [`CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX`]).
/// They stay only as the fixed 64 of the retention bound `MAX_RETAINED_CORPSE_PROJECTIONS` and of
/// the smelly cheese ledger, which no production owner constructs yet.
pub const AI01_SPAWN_SOURCES_PER_SCOPE_MAX: usize = 16;
/// See [`AI01_SPAWN_SOURCES_PER_SCOPE_MAX`].
pub const AI01_SPAWN_POPULATION_MAX: usize = 4;
/// AI01-SPAWN-OCCUPANCY-RETRIES (AI-RL-13): retries per creature per respawn window.
pub const AI01_SPAWN_OCCUPANCY_RETRIES_MAX: u8 = 3;

/// Bounded FIFO retention for `corpse_projections`/`death_reward_occurrences`, which
/// deliberately outlive their actor's removed slot (lost-response replay): the fixed D57
/// envelope of 64 (16 × 4, both factors constant), not the spawn ceilings, so it does not grow
/// with them. Eviction is a defensive bound against unbounded growth over a long-lived
/// Channel's respawn cycles.
const MAX_RETAINED_CORPSE_PROJECTIONS: usize =
    AI01_SPAWN_SOURCES_PER_SCOPE_MAX * AI01_SPAWN_POPULATION_MAX;

/// Codex 4248192466 (LOOT-LIVE-1): the most corpse projections pinned against eviction at once,
/// one per unsettled kill of KILLRW-RL-01 (64, `gameplay_transport::kill_reward`). The bound of
/// `MAX_RETAINED_CORPSE_PROJECTIONS` applies to the unpinned projections only, so
/// `corpse_projections` holds at most 128.
const MAX_PINNED_CORPSE_PROJECTIONS: usize = MAX_RETAINED_CORPSE_PROJECTIONS;

/// A content-authored spawn source's small stable ordinal within its scope (content input,
/// §4.8; never decoded from a client handle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpawnSourceId(pub u16);

/// §4.3/§4.8: one spawn source's content-declared definition. Code never invents these
/// values; a missing one is a content-validation failure at a higher layer, not here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnDefinition {
    /// §4.3: "one placement cell per creature" -- exactly `population` cells, never more
    /// (`placement_cells.len() == population <= CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX`).
    /// Placement never searches beyond these.
    placement_cells: Vec<LocalPosition>,
    /// `1..=CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX`, and exactly `placement_cells.len()`.
    population: usize,
    /// SPAWN-1b (Codex P1 on #1981): what each point spawns, by point ordinal (same order and
    /// length as `placement_cells`). An authored source may mix creatures and respawn delays
    /// across its points, so these are per point, and the source stays one runtime source.
    points: Vec<SpawnPointDefinition>,
    /// D115: 5,000,000 (5 s).
    occupancy_retry_interval_micros: u64,
}

/// SPAWN-1b: what one spawn point realizes and when it respawns.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SpawnPointDefinition {
    /// D115: 60,000,000 (60 s, in the owner clock's microsecond unit).
    respawn_delay_micros: u64,
    creature_target_identity: String,
    /// Always positive.
    creature_initial_health: i64,
    /// CREATURE-AI-0 §6.3: the point's creature profile `blocked_by_nearby_players`. It is per
    /// point because a source may mix blockable and non-blockable creatures (Codex P2 on
    /// #2002). The uniform fixture source is not blockable; only a test definition sets it.
    blockable: bool,
}

/// SPAWN-1b: one point of a source being built: its placement cell and what it spawns.
type SpawnPointBuild = ((i32, i32, i16), SpawnPointDefinition);

impl SpawnDefinition {
    /// A source whose every point spawns the same creature with the same respawn delay.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        placement_cells: Vec<(i32, i32, i16)>,
        population: usize,
        respawn_delay_micros: u64,
        occupancy_retry_interval_micros: u64,
        creature_target_identity: String,
        creature_initial_health: i64,
    ) -> Result<Self, CarrierError> {
        // §4.3: "one declared cell per creature" -- a mismatch either way (fewer cells than
        // the population, or extra unused cells) is invalid, not merely clamped, so
        // `resolve_respawn_timer` can never respawn more than the declared population.
        if population != placement_cells.len() {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        // `CREATUREAI0-RL-03` is checked before one point is built.
        if placement_cells.len() > CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        Self::with_points(
            placement_cells
                .into_iter()
                .map(|cell| {
                    (
                        cell,
                        SpawnPointDefinition {
                            respawn_delay_micros,
                            creature_target_identity: creature_target_identity.clone(),
                            creature_initial_health,
                            blockable: false,
                        },
                    )
                })
                .collect(),
            occupancy_retry_interval_micros,
        )
    }

    /// SPAWN-1b: a source from its points in point-ordinal order, each its cell and what it
    /// spawns. `CREATUREAI0-RL-03` bounds the points; every point's health must be positive.
    fn with_points(
        points: Vec<SpawnPointBuild>,
        occupancy_retry_interval_micros: u64,
    ) -> Result<Self, CarrierError> {
        if points.is_empty() || points.len() > CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let population = points.len();
        let mut placement_cells = Vec::with_capacity(population);
        let mut definitions = Vec::with_capacity(population);
        for ((x, y, floor), point) in points {
            if point.creature_initial_health <= 0 {
                return Err(CarrierError::InvalidSpawnDefinition);
            }
            placement_cells.push(LocalPosition { x, y, floor });
            definitions.push(point);
        }
        Ok(Self {
            placement_cells,
            population,
            points: definitions,
            occupancy_retry_interval_micros,
        })
    }

    /// SPAWN-1a: the activated fixture spawn source as a definition. The cells, population,
    /// delays (ms to the owner clock's µs), creature key and authored initial health all come
    /// from the source; nothing is defaulted here.
    fn from_activation(facts: ActivationSpawnFacts) -> Result<Self, CarrierError> {
        let (cells, respawn_delay_ms, occupancy_retry_interval_ms, creature, initial_health) =
            facts;
        let population = cells.len();
        Self::new(
            cells,
            population,
            activation_micros(respawn_delay_ms)?,
            activation_micros(occupancy_retry_interval_ms)?,
            creature,
            activation_health(initial_health)?,
        )
    }

    /// SPAWN-1b: one activated source of mixed points as a definition, in its authored point
    /// order; nothing is defaulted here.
    fn from_activation_source(facts: ActivationSpawnSourceFacts) -> Result<Self, CarrierError> {
        let (points, occupancy_retry_interval_ms) = facts;
        if points.len() > CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let points = points
            .into_iter()
            .map(
                |(cell, respawn_delay_ms, creature, initial_health, blockable)| {
                    Ok((
                        cell,
                        SpawnPointDefinition {
                            respawn_delay_micros: activation_micros(respawn_delay_ms)?,
                            creature_target_identity: creature,
                            creature_initial_health: activation_health(initial_health)?,
                            blockable,
                        },
                    ))
                },
            )
            .collect::<Result<Vec<_>, CarrierError>>()?;
        Self::with_points(points, activation_micros(occupancy_retry_interval_ms)?)
    }

    /// Every point of this source blockable or not.
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_blockable(mut self, blockable: bool) -> Self {
        for point in &mut self.points {
            point.blockable = blockable;
        }
        self
    }

    /// The definition of point `ordinal`.
    fn point(&self, ordinal: usize) -> Result<&SpawnPointDefinition, CarrierError> {
        self.points
            .get(ordinal)
            .ok_or(CarrierError::UnknownSpawnCell)
    }

    /// The respawn timing of point `ordinal`.
    fn timing(&self, ordinal: usize) -> Result<spawn::PointTiming, CarrierError> {
        let point = self.point(ordinal)?;
        Ok(spawn::PointTiming {
            respawn_delay_micros: point.respawn_delay_micros,
            occupancy_retry_interval_micros: self.occupancy_retry_interval_micros,
            blockable: point.blockable,
        })
    }
}

/// An activation delay in ms as the owner clock's µs.
fn activation_micros(ms: u64) -> Result<u64, CarrierError> {
    ms.checked_mul(1_000)
        .ok_or(CarrierError::InvalidSpawnDefinition)
}

/// An activation initial health as the carrier's signed health.
fn activation_health(health: u64) -> Result<i64, CarrierError> {
    i64::try_from(health).map_err(|_| CarrierError::InvalidSpawnDefinition)
}

/// CREATURE-AI-0 §10: checks one activation against the registered spawn ceilings before
/// anything is reserved or admitted, and returns its total point count. More than
/// `CREATUREAI0-RL-02` sources or more than `CREATUREAI0-RL-01` points refuses with
/// `CapacityExceeded`; `CREATUREAI0-RL-03` is already enforced per source by
/// [`SpawnDefinition::new`]. The work is one pass over the sources.
fn spawn_points_within_ceilings(definitions: &[SpawnDefinition]) -> Result<usize, CarrierError> {
    if definitions.len() > CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX {
        return Err(CarrierError::CapacityExceeded);
    }
    let mut total = 0usize;
    for definition in definitions {
        total = total
            .checked_add(definition.placement_cells.len())
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        if total > CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
    }
    Ok(total)
}

/// SPAWN-1a: one activated spawn source as the carrier reads it: placement cells in order,
/// respawn delay and occupancy retry interval (ms), creature key and authored initial health.
/// Content produces it (`NativeEntrySpawnSource::activation_facts`); Foundation does not name
/// the content type.
pub type ActivationSpawnFacts = (Vec<(i32, i32, i16)>, u64, u64, String, u64);

/// SPAWN-1b (Codex P1 on #1981): one activated spawn point as the carrier reads it: its
/// placement cell, respawn delay (ms), creature key, authored initial health and its creature
/// profile's `blocked_by_nearby_players` (CREATURE-AI-0 §6.3; Codex P2 on #2002), which is per
/// point because one source may mix blockable and non-blockable creatures.
pub type ActivationSpawnPointFacts = ((i32, i32, i16), u64, String, u64, bool);

/// SPAWN-1b (Codex P1 on #1981): one activated spawn source whose points may differ in
/// creature, respawn delay and initial health: its points in authored (point-ordinal) order and
/// its occupancy retry interval (ms). It is one runtime source, so it counts once against
/// `CREATUREAI0-RL-02` and its point ordinals run `0..points` without a reset.
pub type ActivationSpawnSourceFacts = (Vec<ActivationSpawnPointFacts>, u64);

/// SPAWN-1a: what one respawn drive did, in run order. The warning is reported, not
/// broadcast: no client effect exists for it yet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpawnDriveReport {
    pub warnings: Vec<MovementLocalPosition>,
    pub admitted: Vec<ExactActorRef>,
    pub blocked: usize,
    pub retried: usize,
    pub skipped: usize,
}

/// One placement cell's live/pending bookkeeping within a realized spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpawnCellState {
    /// The live creature currently occupying this cell, if any.
    live: Option<ExactActorRef>,
    /// This cell's current respawn chain's attempt index (1-3), 0 before any attempt.
    attempt: u8,
    /// Bumped each time an occurrence chain ends `Skipped` and a fresh one starts (§4.3
    /// "terminal disposition"): keeps every occurrence identity for this cell unique.
    successor: u32,
}

/// One realized spawn source and its per-cell state, for this carrier's ownership
/// generation (§4.3: `EphemeralScopeReset` -- a restart/scope-move realizes it again; nothing
/// here is durable).
#[derive(Debug, Clone, PartialEq, Eq)]
struct SpawnRealization {
    source: SpawnSourceId,
    definition: SpawnDefinition,
    /// Same order/length as `definition.placement_cells`.
    cells: Vec<SpawnCellState>,
}

/// AI-RL for respawn timers (§4.2's table; no separate `RESOURCE_LIMITS_REGISTRY.json` row is
/// named for it distinctly from AI01-PENDING-TIMERS-PER-ACTOR): at most one pending respawn
/// timer per dead creature (`OwnerTimerLane`'s `target` key), which bounds the aggregate
/// pending count for one spawn to its own population (§4.9's derived "at most the spawn's
/// population pending").
pub const AI01_PENDING_RESPAWN_TIMERS_PER_DEAD_ACTOR: usize = 1;

/// AI-2's one timer family: respawn (§4.2's table: `DEADLINE_STATE`, distinct from AI-1's
/// `SKIP_TO_LATEST` think family -- every due retry/successor must still fire, never collapsed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnFamily {
    Respawn,
}

impl super::owner_timer::TimerFamily for RespawnFamily {
    fn registered_maximum(self) -> usize {
        match self {
            Self::Respawn => AI01_PENDING_RESPAWN_TIMERS_PER_DEAD_ACTOR,
        }
    }
}

/// A respawn occurrence identity (§4.2: "(spawn source, cell, the dead actor's
/// `ExactActorRef`), and each retry adds its attempt index 1 to 3"; §4.3's terminal
/// disposition adds a successor index for the chain that follows a `Skipped` occurrence).
/// Binding item (a) (bounded replay evidence, carried over from AI-1): `attempt` and
/// `successor` only ever increase for a given `(source, cell_index, dead_actor)` (enforced by
/// `resolve_respawn_timer`, the only place that mints one), so the full tuple can never repeat
/// in a lane's lifetime -- bounded, O(1) state per cell (`SpawnCellState`), never an unbounded
/// log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespawnOccurrence {
    pub source: SpawnSourceId,
    pub cell_index: u8,
    pub dead_actor: ExactActorRef,
    pub successor: u32,
    pub attempt: u8,
}

/// The result of applying one fired respawn timer (§4.3). This never mutates state itself
/// beyond `ChannelActorCarrier::resolve_respawn_timer`'s own admission/bookkeeping; scheduling
/// the next timer (if any) is the caller's job, exactly as `OwnerTimerLane` requires (binding
/// item (b): the caller's own fence-issued `RuntimeWorkStamp`, never fabricated here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnResolution {
    /// A new creature was admitted at the cell with a fresh actor-local generation (D52: the
    /// dead actor's generation is never reused).
    Admitted(ExactActorRef),
    /// The cell was occupied; the caller schedules one fresh occurrence with `next_attempt`,
    /// due `occupancy_retry_interval_micros` later.
    Postponed { next_attempt: u8 },
    /// The third retry also found the cell occupied: this chain ends `Skipped`, nothing
    /// admitted. The caller schedules one new successor occurrence (`next_successor`, attempt
    /// 1) due one full `respawn_delay_micros` later.
    Skipped { next_successor: u32 },
}

/// D2b: mints a fresh v7-shaped `ExperienceRewardOccurrence` byte pattern
/// (RFC 9562 version/variant nibbles; the remaining bits are a SHA-256
/// digest of the death's exact actor reference, the reward principal and the
/// current wall-clock millisecond, so two calls never collide). Called at
/// most once per (death, character): `reward_occurrence_inner` memoizes the
/// first result and every later call returns that same value.
fn mint_reward_occurrence_bytes(actor: ActorRef, character: [u8; 16]) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);
    let digest = Sha256::new()
        .chain_update(b"oteryn:combat-death-xp-occurrence:v1")
        .chain_update(actor.world_id.as_bytes())
        .chain_update(actor.channel_id.as_bytes())
        .chain_update(actor.scope_generation.get().to_be_bytes())
        .chain_update(actor.actor_local_id.0.to_be_bytes())
        .chain_update(actor.actor_local_generation.0.to_be_bytes())
        .chain_update(character)
        .chain_update(millis.to_be_bytes())
        .finalize();
    let mut bytes = [0_u8; 16];
    bytes[0..6].copy_from_slice(&millis.to_be_bytes()[2..8]);
    bytes[6] = 0x70 | (digest[0] & 0x0f);
    bytes[7] = digest[1];
    bytes[8] = 0x80 | (digest[2] & 0x3f);
    bytes[9..16].copy_from_slice(&digest[3..10]);
    bytes
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

impl ChannelRuntimeV1 {
    /// CREATURE-AI-1: the health of live creature `actor`.
    pub fn live_creature_health(&self, actor: ExactActorRef) -> Option<u64> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0).ok()?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                generation, health, ..
            } if *generation == actor.0.actor_local_generation.0 && *health > 0 => {
                u64::try_from(*health).ok()
            }
            _ => None,
        }
    }

    /// Test only: sets a live creature's health directly.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_test_creature_health(
        &mut self,
        actor: ExactActorRef,
        value: i64,
    ) -> Result<(), CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &mut self.carrier.slots[index] {
            Slot::CreatureOccupied { health, .. } => {
                *health = value;
                Ok(())
            }
            _ => Err(CarrierError::NotCreature),
        }
    }
}

/// Outcome of a first-entry position initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstEntryPosition {
    /// This call wrote the start position.
    Initialized(MovementPositionSnapshot),
    /// The same initialization had already completed; nothing was written.
    Reconciled(MovementPositionSnapshot),
}

/// The first composed Channel runtime. The fixed-slot carrier is the only actor
/// resource: no session map or second index is introduced.
#[derive(Debug)]
pub struct ChannelRuntimeV1 {
    binding: ChannelRuntimeAssignmentBinding,
    continuity: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    content: ChannelContentPin,
    companion_policies: Option<Arc<runtime_actor_companion::CompiledCreaturePolicies>>,
    owner_cycle: super::ScopeRuntimeFence,
    /// SPAWN-1a: the pending respawn occurrences of the realized spawn sources (§6.3).
    spawn_schedule: spawn::RespawnSchedule,
    /// SPAWN-CEILINGS-1 (Codex 4243397472): the occurrences of `spawn_schedule` by (deadline,
    /// point), written with every schedule write, so a drive reads only the occurrences it runs
    /// (`O(log n)` each) instead of collecting and sorting every due occurrence.
    spawn_deadlines: BTreeSet<(u64, spawn::SpawnPoint)>,
    /// SPAWN-CEILINGS-1 (Codex 4243397472): the first point the next gone scan reads
    /// ([`SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX`] per owner window, wrapping).
    spawn_scan_cursor: spawn::SpawnPoint,
    /// SPAWN-CEILINGS-1: an activation larger than one owner window, realized one window per
    /// owner window (`CREATUREAI0-RL-14`); `None` when no activation is in progress.
    spawn_activation: Option<Box<SpawnActivation>>,
    /// SPAWN-CEILINGS-1: the owner window of the last gone scan and its remaining step budget,
    /// so the scan runs once per Channel window, not once per drive.
    spawn_drive_window: Option<SpawnDriveWindow>,
    /// Test only: the spawn records the respawn drives read (gone-scan points and deadline
    /// entries), to prove the work per owner window is bounded.
    #[cfg(any(test, feature = "test-support"))]
    spawn_drive_visits: usize,
}

/// SPAWN-CEILINGS-1 (CREATURE-AI-0 §6.2, `CREATUREAI0-RL-14`): an activation in progress: its
/// points in canonical order (source ordinal, point ordinal), split by
/// [`spawn::realization_windows`] into owner windows of at most
/// [`spawn::REALIZATIONS_PER_WINDOW_MAX`], and the cursor of the next window.
#[derive(Debug)]
struct SpawnActivation {
    /// The definitions by source ordinal; a source's entry is taken at its point 0.
    definitions: Vec<Option<SpawnDefinition>>,
    /// The realization plan; a realized window is emptied.
    windows: Vec<Vec<spawn::SpawnPoint>>,
    /// The next window to realize: the persisted cursor.
    next_window: usize,
    /// Points realized so far.
    realized_points: usize,
    /// Points still to realize.
    remaining_points: usize,
    /// The owner window (`now / OWNER_WINDOW_MICROS`) that realized the last window.
    last_window: Option<u64>,
}

impl SpawnActivation {
    fn new(definitions: Vec<SpawnDefinition>, total_points: usize) -> Result<Self, CarrierError> {
        let mut points = Vec::new();
        points
            .try_reserve_exact(total_points)
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (source, definition) in definitions.iter().enumerate() {
            let source = u16::try_from(source).map_err(|_| CarrierError::CapacityExceeded)?;
            for ordinal in 0..definition.placement_cells.len() {
                let ordinal = u8::try_from(ordinal).map_err(|_| CarrierError::CapacityExceeded)?;
                points.push(spawn::SpawnPoint { source, ordinal });
            }
        }
        let windows = spawn::realization_windows(&points);
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(definitions.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        pending.extend(definitions.into_iter().map(Some));
        Ok(Self {
            definitions: pending,
            windows,
            next_window: 0,
            realized_points: 0,
            remaining_points: total_points,
            last_window: None,
        })
    }

    const fn progress(&self, realized: usize) -> SpawnActivationProgress {
        SpawnActivationProgress {
            realized,
            realized_total: self.realized_points,
            remaining: self.remaining_points,
        }
    }
}

/// SPAWN-CEILINGS-1: what one activation window call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnActivationProgress {
    /// Creatures this call realized: 0 when its owner window had already realized one.
    pub realized: usize,
    /// Creatures the activation has realized so far.
    pub realized_total: usize,
    /// Points still to realize; 0 means the activation is complete.
    pub remaining: usize,
}

/// SPAWN-CEILINGS-1: one owner window of the respawn drive. The first drive of a window runs
/// the window's gone scan; every drive of it then runs the occurrences whose deadlines have
/// passed, from the deadline index, within the window's step budget.
#[derive(Debug)]
struct SpawnDriveWindow {
    window: u64,
    /// Steps this window may still run ([`SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX`]).
    steps_left: usize,
}

impl ChannelRuntimeV1 {
    /// Issue from the one non-clonable owner cycle installed by the independently committed
    /// assignment. A retired namespace can neither schedule nor commit further actor work.
    pub fn issue_owner_work(&mut self) -> Result<super::RuntimeWorkStamp, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        let ordinal = self
            .owner_cycle
            .accept_input(self.binding.scope_generation)
            .map_err(|_| CarrierError::WrongScope)?;
        Ok(self.owner_cycle.stamp(ordinal))
    }

    pub fn owner_fence(&self) -> Result<&super::ScopeRuntimeFence, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        if !self.owner_cycle.is_current_for_scope(
            super::RuntimeScopeRefV1::channel(self.binding.world_id, self.binding.channel_id),
            self.binding.scope_generation,
        ) {
            return Err(CarrierError::WrongScope);
        }
        Ok(&self.owner_cycle)
    }

    /// Denial only, called when the existing assignment consumer observes retirement/handoff.
    /// A successor obtains a new runtime from its independently committed assignment.
    pub fn retire_owner_cycle(&mut self) {
        self.owner_cycle.invalidate();
        self.continuity.active = false;
    }
    #[allow(clippy::too_many_arguments)]
    pub fn from_committed_assignment(
        world_id: WorldId,
        channel_id: ChannelId,
        node_id: NodeId,
        node_registration_revision: u64,
        ownership_generation: u64,
        source_revision: u64,
        decision_identity: &str,
        explicit_capacity: usize,
        content: ChannelContentPin,
    ) -> Result<Self, CarrierError> {
        if node_registration_revision == 0 || source_revision == 0 {
            return Err(CarrierError::InvalidAssignmentBinding);
        }
        if content.world_id != world_id {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let decision_revision = decision_identity
            .strip_prefix("runtime-scope-assignment:")
            .and_then(|value| value.parse::<u64>().ok());
        if decision_revision != Some(source_revision) {
            return Err(CarrierError::InvalidAssignmentBinding);
        }
        let scope_generation = ScopeOwnershipGeneration::new(ownership_generation)
            .map_err(|_| CarrierError::InvalidAssignmentBinding)?;
        let grant = PreProductionContinuityGrant {
            world_id,
            channel_id,
            scope_generation,
        };
        let mut continuity = NamespaceContinuityGuard::from_pre_production_grant(grant);
        let carrier =
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity, explicit_capacity)?;
        Ok(Self {
            binding: ChannelRuntimeAssignmentBinding {
                world_id,
                channel_id,
                node_id,
                node_registration_revision,
                scope_generation,
                source_revision,
            },
            continuity,
            carrier,
            content,
            companion_policies: None,
            owner_cycle: super::ScopeRuntimeFence::from_external_grant(scope_generation)
                .with_scope(super::RuntimeScopeRefV1::channel(world_id, channel_id)),
            spawn_schedule: spawn::RespawnSchedule::default(),
            spawn_deadlines: BTreeSet::new(),
            spawn_scan_cursor: spawn::SpawnPoint {
                source: 0,
                ordinal: 0,
            },
            spawn_activation: None,
            spawn_drive_window: None,
            #[cfg(any(test, feature = "test-support"))]
            spawn_drive_visits: 0,
        })
    }

    pub const fn binding(&self) -> ChannelRuntimeAssignmentBinding {
        self.binding
    }

    pub const fn content_pin(&self) -> &ChannelContentPin {
        &self.content
    }

    /// Unactivated Movement proof: one exclusive borrow of the composed Channel owner.
    /// This grants neither initial position authority nor an owner scheduler.
    pub fn borrow_movement_position(&mut self) -> CurrentOwnerMovementPosition<'_> {
        self.carrier
            .current_owner_movement_position(&self.continuity)
    }

    /// The Movement owner context of this runtime's fixed Content pin; a step is eligible only
    /// for a position initialized under exactly this context.
    pub fn pinned_movement_context(&self) -> MovementPositionContext {
        MovementPositionContext(self.pinned_position_context())
    }

    /// Current-owner census (#162 5868482467, shared-lease P1 repair r4121956127): the local
    /// positions of every committed, currently-present player actor under this runtime's pinned
    /// Movement context — the same per-slot position this type's own `read()` reads for one
    /// actor, read here for all of them in one Channel-owner work item. No parallel tracking of
    /// actor positions is introduced: this reads the existing `slots` store directly, exactly as
    /// the existing test-only census methods below do, and reveals no slot index or generation to
    /// its caller. A world-object occupancy check intersects these positions' cells with a
    /// LocalObject's own collision footprint; it must read them under the same runtime lock and
    /// the same work item as the transition it gates, so the check is TOCTOU-free.
    /// Actual fixed-slot occupancy, including living creatures. Missing/foreign-context
    /// positions are unknown and refuse traversal; they are never interpreted as clear.
    pub fn position_occupied_by_other(
        &self,
        actor: ExactActorRef,
        destination: MovementLocalPosition,
    ) -> Result<bool, CarrierError> {
        let source_index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let context = self.pinned_position_context();
        for (index, slot) in self.carrier.slots.iter().enumerate() {
            if index == source_index {
                continue;
            }
            let spell_combat = match slot {
                Slot::Occupied { spell_combat, .. }
                | Slot::CreatureOccupied { spell_combat, .. } => Some(spell_combat),
                _ => None,
            };
            if spell_combat
                .and_then(|state| state.pending_owner.as_ref())
                .and_then(|reservation| reservation.destination())
                .is_some_and(|(relocated, reserved)| relocated != actor && reserved == destination)
            {
                return Ok(true);
            }
            let version = match slot {
                Slot::Occupied {
                    committed: true,
                    position,
                    ..
                } => *position,
                Slot::CreatureOccupied {
                    health, position, ..
                } if *health > 0 => *position,
                Slot::CreatureReserved { planned, .. } => match planned.as_ref() {
                    Slot::CreatureOccupied { position, .. } => *position,
                    _ => return Err(CarrierError::PlanConflict),
                },
                Slot::NpcOccupied { position, .. } => Some(*position),
                _ => continue,
            }
            .ok_or(CarrierError::PositionSnapshotMismatch)?;
            if version.context != context {
                return Err(CarrierError::PositionContextMismatch);
            }
            if version.position
                == (LocalPosition {
                    x: destination.x,
                    y: destination.y,
                    floor: destination.floor,
                })
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether `actor`'s own unresolved spell reservation relocates somebody into `cell`.
    /// `position_occupied_by_other` skips the issuer's slot, so a world-object gate that must
    /// not close a tile onto the issuer's pending destination asks this as well.
    pub fn reserved_into_by_own_pending(
        &self,
        actor: ExactActorRef,
        cell: MovementLocalPosition,
    ) -> Result<bool, CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        Ok(match self.carrier.slots.get(index) {
            Some(
                Slot::Occupied { spell_combat, .. } | Slot::CreatureOccupied { spell_combat, .. },
            ) => spell_combat
                .pending_owner
                .as_ref()
                .and_then(|reservation| reservation.destination())
                .is_some_and(|(_, reserved)| reserved == cell),
            _ => false,
        })
    }

    pub fn committed_player_positions(&self) -> Vec<MovementLocalPosition> {
        let context = self.pinned_position_context();
        self.carrier
            .slots
            .iter()
            .filter_map(|slot| match slot {
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    position: Some(version),
                    ..
                } if version.context == context => Some(MovementLocalPosition {
                    x: version.position.x,
                    y: version.position.y,
                    floor: version.position.floor,
                }),
                _ => None,
            })
            .collect()
    }

    /// VIS-3 (MOVE-RL-11 §4.2, D524): every entity this Channel shows in
    /// `WORLD_SPATIAL_VISIBILITY` under the pinned Movement context, read in one owner work item:
    /// committed present players, live creatures and placed NPCs. Corpses wait on their item binding (D3-7).
    /// Read-only: a value snapshot of the existing `slots`, never an authority token.
    pub fn visible_entities(&self) -> VisibleRuntimeEntities {
        let context = self.pinned_position_context();
        let carrier = &self.carrier;
        let actor = |version: &VersionedPosition| VisibleRuntimeActor {
            actor: ExactActorRef(ActorRef {
                world_id: carrier.world_id,
                channel_id: carrier.channel_id,
                scope_generation: carrier.scope_generation,
                actor_local_id: version.actor_local_id,
                actor_local_generation: version.actor_local_generation,
            }),
            generation: version.actor_local_generation.0,
            position: MovementLocalPosition {
                x: version.position.x,
                y: version.position.y,
                floor: version.position.floor,
            },
            revision: version.revision,
        };
        let mut visible = VisibleRuntimeEntities::default();
        // Only the occupied slots (Codex 4178196302): never a scan of the whole capacity.
        let occupied = carrier
            .occupied
            .iter()
            .filter_map(|index| carrier.slots.get(usize::try_from(*index).ok()?));
        for slot in occupied {
            match slot {
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    position: Some(version),
                    ..
                } if version.context == context => visible.players.push(actor(version)),
                Slot::CreatureOccupied {
                    health,
                    position: Some(version),
                    ..
                } if *health > 0 && version.context == context => {
                    visible.creatures.push(actor(version));
                }
                Slot::NpcOccupied {
                    position: version, ..
                } if version.context == context => visible.npcs.push(actor(version)),
                _ => {}
            }
        }
        visible
    }

    /// The carrier's slots: every actor of this runtime, of any kind, takes one. A ledger with
    /// at most one entry per slot is bounded by it.
    pub fn actor_slot_capacity(&self) -> usize {
        self.carrier.slots.len()
    }

    /// AI-4 (GAME-AI-01 slice §4.6): true only while `actor` names a live (`health > 0`)
    /// creature generation of this runtime. A player, a stale or dead generation, a vacant slot
    /// or another scope is false.
    pub fn contains_live_creature(&self, actor: ExactActorRef) -> bool {
        let Ok(index) = self.carrier.validate_ref(&self.continuity, actor.0) else {
            return false;
        };
        matches!(
            &self.carrier.slots[index],
            Slot::CreatureOccupied { generation, health, .. }
                if *generation == actor.0.actor_local_generation.0 && *health > 0
        )
    }

    /// AI-4 (§4.6): the current position of a live creature or committed player actor, read in
    /// the owner work item that uses it. A value snapshot, never an authority token.
    pub fn read_actor_position(
        &self,
        actor: ExactActorRef,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .read_position(&self.continuity, actor.0)
            .map(MovementPositionSnapshot)
    }

    /// Complete bounded census of the owner's positioned committed players and
    /// live creatures. These current value snapshots grant no mutation authority;
    /// each consumer must revalidate exact actors, session and positions at commit.
    pub fn positioned_actor_census(
        &self,
    ) -> Result<
        Vec<(
            ExactActorRef,
            MovementPositionSnapshot,
            Option<GameSessionId>,
        )>,
        CarrierError,
    > {
        self.census(false)
    }

    /// ATTACK0-RL-03: the creature melee census. It also lists committed players whose control
    /// was lost and who stay positioned; the consumer admits only those still in fight.
    pub fn positioned_melee_census(
        &self,
    ) -> Result<
        Vec<(
            ExactActorRef,
            MovementPositionSnapshot,
            Option<GameSessionId>,
        )>,
        CarrierError,
    > {
        self.census(true)
    }

    fn census(
        &self,
        include_control_lost: bool,
    ) -> Result<
        Vec<(
            ExactActorRef,
            MovementPositionSnapshot,
            Option<GameSessionId>,
        )>,
        CarrierError,
    > {
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.owner_fence()?;
        // SPAWN-CEILINGS-1 (Codex 4242381774): the census reads the occupied index in slot order,
        // so a pass costs the actors present, not the capacity, and its order is unchanged.
        let mut indexes = Vec::new();
        indexes
            .try_reserve_exact(self.carrier.occupied.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        indexes.extend_from_slice(&self.carrier.occupied);
        indexes.sort_unstable();
        let mut result = Vec::new();
        result
            .try_reserve(indexes.len())
            .map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        for index in indexes {
            let index =
                usize::try_from(index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
            let Some(slot) = self.carrier.slots.get(index) else {
                continue;
            };
            let (generation, session) = match slot {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(session),
                    committed: true,
                    lifecycle,
                    position: Some(_),
                    ..
                } if include_control_lost || lifecycle.control_loss.is_none() => {
                    (*generation, Some(*session))
                }
                Slot::CreatureOccupied {
                    generation,
                    health,
                    position: Some(_),
                    ..
                } if *health > 0 => (*generation, None),
                _ => continue,
            };
            let actor = ExactActorRef(ActorRef {
                world_id: self.binding.world_id,
                channel_id: self.binding.channel_id,
                scope_generation: self.binding.scope_generation,
                actor_local_id: ActorLocalId(
                    u32::try_from(index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                actor_local_generation: ActorLocalGeneration(generation),
            });
            let position = self.read_actor_position(actor)?;
            if position.context() != self.pinned_movement_context() {
                return Err(CarrierError::PositionContextMismatch);
            }
            result.push((actor, position, session));
        }
        Ok(result)
    }

    /// Revalidate a previously staged complete census without allocating after COMMIT.
    pub fn validate_positioned_actor_census(
        &self,
        expected: &[(
            ExactActorRef,
            MovementPositionSnapshot,
            Option<GameSessionId>,
        )],
    ) -> Result<(), CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.owner_fence()?;
        let mut cursor = 0usize;
        for (index, slot) in self.carrier.slots.iter().enumerate() {
            let (generation, session) = match slot {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(session),
                    committed: true,
                    lifecycle,
                    position: Some(_),
                    ..
                } if lifecycle.control_loss.is_none() => (*generation, Some(*session)),
                Slot::CreatureOccupied {
                    generation,
                    health,
                    position: Some(_),
                    ..
                } if *health > 0 => (*generation, None),
                _ => continue,
            };
            let actor = ExactActorRef(ActorRef {
                world_id: self.binding.world_id,
                channel_id: self.binding.channel_id,
                scope_generation: self.binding.scope_generation,
                actor_local_id: ActorLocalId(
                    u32::try_from(index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                actor_local_generation: ActorLocalGeneration(generation),
            });
            let position = self.read_actor_position(actor)?;
            if position.context() != self.pinned_movement_context() {
                return Err(CarrierError::PositionContextMismatch);
            }
            if expected.get(cursor) != Some(&(actor, position, session)) {
                return Err(CarrierError::PlayerReservationMismatch);
            }
            cursor += 1;
        }
        if cursor != expected.len() {
            return Err(CarrierError::PlayerReservationMismatch);
        }
        Ok(())
    }

    /// Local, bounded owner lookup for an independently resolved current GameSession.
    /// Names and privacy permission are resolved by their owners before using this read.
    pub fn positioned_player_for_session(
        &self,
        session: GameSessionId,
    ) -> Result<Option<(ExactActorRef, MovementPositionSnapshot)>, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        let mut result = None;
        for (index, slot) in self.carrier.slots.iter().enumerate() {
            let Slot::Occupied {
                generation,
                game_session_id: Some(current),
                committed: true,
                lifecycle,
                position: Some(_),
                ..
            } = slot
            else {
                continue;
            };
            if lifecycle.control_loss.is_some() || *current != session {
                continue;
            }
            if result.is_some() {
                return Err(CarrierError::PlayerReservationMismatch);
            }
            let actor = ExactActorRef(ActorRef {
                world_id: self.binding.world_id,
                channel_id: self.binding.channel_id,
                scope_generation: self.binding.scope_generation,
                actor_local_id: ActorLocalId(
                    u32::try_from(index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                actor_local_generation: ActorLocalGeneration(*generation),
            });
            let position = self.read_actor_position(actor)?;
            if position.context() != self.pinned_movement_context() {
                return Err(CarrierError::PositionContextMismatch);
            }
            result = Some((actor, position));
        }
        Ok(result)
    }

    /// Unactivated Combat proof: one exclusive borrow of the physical Channel
    /// owner. It grants no scheduler, production activation or corpse lifetime.
    pub fn borrow_combat_death(&mut self) -> CurrentOwnerCombatDeath<'_> {
        self.carrier.current_owner_combat_death(&self.continuity)
    }

    /// Borrow the same physical owner used by Ability resolution. A snapshot produced by
    /// this lookup never substitutes for the current checks at the later commit boundary.
    pub fn borrow_exact_actor_lookup(&self) -> CurrentOwnerExactActorLookup<'_> {
        self.carrier.current_owner_exact_lookup(&self.continuity)
    }

    /// Borrow the existing generation-fenced creature HP owner with this runtime's
    /// immutable current Content pin; registered source encounters revalidate it.
    pub fn borrow_exact_actor_commit(&mut self) -> CurrentOwnerExactActorCommit<'_> {
        CurrentOwnerExactActorCommit {
            carrier: &mut self.carrier,
            continuity: &self.continuity,
            content: Some(&self.content),
            now_ms: None,
        }
    }

    /// [`Self::borrow_exact_actor_commit`] for an owner turn whose clock is `now_ms`: its
    /// commits record that time as the damage and, for a lethal commit, the death time.
    pub fn borrow_exact_actor_commit_at(
        &mut self,
        now_ms: u64,
    ) -> CurrentOwnerExactActorCommit<'_> {
        CurrentOwnerExactActorCommit {
            carrier: &mut self.carrier,
            continuity: &self.continuity,
            content: Some(&self.content),
            now_ms: Some(now_ms),
        }
    }

    /// The compact position context of this runtime's fixed Content pin.
    fn pinned_position_context(&self) -> PreProductionPositionContext {
        let prefix = |digest: &[u8; 32]| {
            u64::from_be_bytes([
                digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6],
                digest[7],
            ])
        };
        PreProductionPositionContext {
            world_id: self.binding.world_id,
            channel_id: self.binding.channel_id,
            scope_generation: self.binding.scope_generation,
            coordinate_frame_marker: prefix(&self.content.frame_binding_digest),
            map_revision_marker: prefix(&self.content.map_revision_digest),
            content_generation_marker: self.content.activation_sequence,
        }
    }

    /// #935 first-entry position initialization: the Channel owner writes the
    /// pinned generation's start cell for one committed, unpositioned player
    /// actor. The context comes only from this runtime's assignment binding and
    /// Content pin. An exact retry reconciles the completed initialization
    /// without another write (later Movement is not undone); a stale or
    /// recycled actor, or one positioned under another context, gets no write.
    pub fn initialize_first_entry_position(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<FirstEntryPosition, CarrierError> {
        let context = self.pinned_position_context();
        match self.carrier.initialize_position(
            &self.continuity,
            actor.0,
            context,
            self.content.entry_start,
        ) {
            Ok(snapshot) => Ok(FirstEntryPosition::Initialized(MovementPositionSnapshot(
                snapshot,
            ))),
            Err(CarrierError::PositionAlreadyInitialized) => {
                let current = self.carrier.read_position(&self.continuity, actor.0)?;
                if current.version.context == context {
                    Ok(FirstEntryPosition::Reconciled(MovementPositionSnapshot(
                        current,
                    )))
                } else {
                    Err(CarrierError::PositionAlreadyInitialized)
                }
            }
            Err(error) => Err(error),
        }
    }

    /// Facts of the committed player actor bound to exactly this GameSession.
    pub fn player_control_facts(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Result<PlayerControlFacts, CarrierError> {
        let control_loss =
            self.carrier
                .player_control_loss(&self.continuity, actor.0, game_session_id)?;
        Ok(PlayerControlFacts {
            placement_identity: actor.placement_identity(),
            placement_revision: actor.0.actor_local_generation.0,
            control_loss,
        })
    }

    /// A2: the current bound lease of `actor`'s player slot held by `game_session_id`, read
    /// without a write (ATTACK-1b Charm hooks). Same checks as the commit-time read.
    pub fn bound_attacker_lease(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Result<CharacterLease, CarrierError> {
        self.carrier
            .bound_attacker_lease(&self.continuity, actor.0, game_session_id)
    }

    /// Mirror one committed durable control loss onto the still-present player actor.
    /// Recording the identical decision again is a no-op; a different one conflicts.
    pub fn record_control_loss(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        mark: ControlLossMark,
    ) -> Result<(), CarrierError> {
        self.carrier
            .record_player_control_loss(&self.continuity, actor.0, game_session_id, mark)
    }

    /// Clear the exact recorded loss once the durable same-session recovery committed:
    /// the still-present actor is controlled again. Idempotent when already restored; a
    /// different recorded epoch is a conflict.
    pub fn restore_control(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        epoch: u64,
    ) -> Result<(), CarrierError> {
        self.carrier
            .restore_player_control(&self.continuity, actor.0, game_session_id, epoch)
    }

    /// Synthetic context is confined to tests. A committed session still goes through
    /// the actual runtime reservation and carrier position initialization checks.
    #[cfg(any(test, feature = "test-support"))]
    pub fn initialize_movement_test_position(
        &mut self,
        actor: ExactActorRef,
        position: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let context = self.test_position_context();
        self.carrier
            .initialize_position(
                &self.continuity,
                actor.0,
                context,
                LocalPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
            )
            .map(MovementPositionSnapshot)
    }

    /// Test only: `initialize_movement_test_position` under the active pin's position context,
    /// for consumers that read occupancy or commit steps beside pinned creatures.
    #[cfg(any(test, feature = "test-support"))]
    pub fn initialize_pinned_test_position(
        &mut self,
        actor: ExactActorRef,
        position: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let context = self.pinned_position_context();
        self.carrier
            .initialize_position(
                &self.continuity,
                actor.0,
                context,
                LocalPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
            )
            .map(MovementPositionSnapshot)
    }

    #[cfg(any(test, feature = "test-support"))]
    const fn test_position_context(&self) -> PreProductionPositionContext {
        PreProductionPositionContext {
            world_id: self.binding.world_id,
            channel_id: self.binding.channel_id,
            scope_generation: self.binding.scope_generation,
            coordinate_frame_marker: 11,
            map_revision_marker: 12,
            content_generation_marker: 13,
        }
    }

    /// Explicit Game-owned qualification declaration. The caller independently holds
    /// current qualified tile and SQL Item collision proofs through this owner turn.
    /// Health and policy come from this active runtime, not fixture health constants.
    #[cfg(any(test, feature = "test-support"))]
    pub fn realize_native_qualification_spawn(
        &mut self,
        expected_pin: ChannelContentPin,
        definition_key: &str,
        position: MovementLocalPosition,
    ) -> Result<runtime_actor_companion::CompanionSnapshot, CarrierError> {
        self.owner_fence()?;
        if self.spawn_activation.is_some() {
            return Err(CarrierError::PlanConflict);
        }
        if self.content != expected_pin {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let policy = self.companion_policy(definition_key)?;
        if policy.is_familiar {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        // The census refuses foreign physical contexts; reserved/pending destinations
        // are checked separately so qualification cannot displace another owner.
        self.positioned_actor_census()?;
        let context = self.pinned_position_context();
        let cell = LocalPosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        if self.carrier.cell_occupied(context, cell)
            || self.carrier.slots.iter().any(|slot| match slot {
                Slot::Occupied {
                    committed: true,
                    position: None,
                    ..
                }
                | Slot::CreatureOccupied { position: None, .. }
                | Slot::Occupied {
                    committed: false, ..
                } => true,
                Slot::Occupied { spell_combat, .. }
                | Slot::CreatureOccupied { spell_combat, .. } => spell_combat
                    .pending_owner
                    .as_ref()
                    .and_then(|p| p.destination())
                    .is_some_and(|(_, destination)| destination == position),
                _ => false,
            })
        {
            return Err(CarrierError::PlanConflict);
        }
        let definition = SpawnDefinition::new(
            vec![(position.x, position.y, position.floor)],
            1,
            60_000_000,
            5_000_000,
            policy.definition_key.clone(),
            policy.maximum_health,
        )?;
        let slots_before = self.carrier.slots.clone();
        let free_before = self.carrier.free_head;
        let occupied_before = self.carrier.occupied.clone();
        let spawns_before = self.carrier.spawns.clone();
        let result = (|| {
            self.carrier.realize_spawn(
                &self.continuity,
                SpawnSourceId(u16::MAX),
                definition,
                context,
            )?;
            let actor = self
                .carrier
                .spawn_cell_state(SpawnSourceId(u16::MAX), 0)?
                .live
                .ok_or(CarrierError::PlanConflict)?;
            self.install_creature_policy(actor, definition_key)?;
            self.companion_snapshot(actor)
        })();
        if result.is_err() {
            self.carrier.slots = slots_before;
            self.carrier.free_head = free_before;
            self.carrier.occupied = occupied_before;
            self.carrier.spawns = spawns_before;
        }
        result
    }

    /// SPAWN-1a (CREATURE-AI-0 §6.2): realizes the activated spawn sources at Channel
    /// activation, before the Channel admits players, in one owner turn. Sources are realized
    /// in canonical order (source ordinal `i`, point ordinal = cell index), each creature a
    /// fresh actor-local generation with the source's authored initial health. A creature
    /// whose definition the active Creature policy table names is bound to that policy;
    /// otherwise it stands unbound. Any refusal realizes nothing. Returns the number of
    /// creatures realized.
    ///
    /// One owner turn realizes at most one owner window, `CREATUREAI0-RL-14`
    /// ([`spawn::REALIZATIONS_PER_WINDOW_MAX`] points): a larger activation refuses here with
    /// `CapacityExceeded` and nothing realized, and runs through
    /// [`Self::begin_activation_spawns`] and [`Self::realize_activation_spawn_window`] instead.
    pub fn realize_activation_spawns(
        &mut self,
        sources: Vec<ActivationSpawnFacts>,
    ) -> Result<usize, CarrierError> {
        let total_points = self.begin_activation_spawns(sources)?;
        self.realize_begun_activation_in_one_turn(total_points)
    }

    #[cfg(any(test, feature = "test-support"))]
    fn realize_spawn_definitions(
        &mut self,
        definitions: Vec<SpawnDefinition>,
    ) -> Result<usize, CarrierError> {
        let total_points = self.begin_spawn_definitions(definitions)?;
        self.realize_begun_activation_in_one_turn(total_points)
    }

    /// Completes the activation just begun in this owner turn, which may hold at most one
    /// owner window of points; a larger one is dropped unadmitted and refuses
    /// `CapacityExceeded`. The window is staged under one snapshot: a failure partway (for
    /// example `ActorGenerationExhausted`, `admit_inner`) restores it exactly.
    fn realize_begun_activation_in_one_turn(
        &mut self,
        total_points: usize,
    ) -> Result<usize, CarrierError> {
        if total_points > spawn::REALIZATIONS_PER_WINDOW_MAX {
            self.spawn_activation = None;
            return Err(CarrierError::CapacityExceeded);
        }
        let slots_before = self.carrier.slots.clone();
        let free_before = self.carrier.free_head;
        let occupied_before = self.carrier.occupied.clone();
        match self.realize_activation_spawn_window(0) {
            Ok(progress) => {
                debug_assert_eq!(progress.remaining, 0);
                Ok(progress.realized)
            }
            Err(error) => {
                self.spawn_activation = None;
                self.carrier.slots = slots_before;
                self.carrier.free_head = free_before;
                self.carrier.occupied = occupied_before;
                self.carrier.spawns.clear();
                Err(error)
            }
        }
    }

    /// SPAWN-CEILINGS-1 (CREATURE-AI-0 §6.2, `CREATUREAI0-RL-14`): checks an activation of any
    /// size up to the registered ceilings and keeps it as a persisted cursor; nothing is
    /// admitted yet. [`Self::realize_activation_spawn_window`] then realizes one owner window
    /// per call, and the Channel admits players only after it reports nothing remaining.
    /// Returns the activation's total points. A refusal changes nothing.
    pub fn begin_activation_spawns(
        &mut self,
        sources: Vec<ActivationSpawnFacts>,
    ) -> Result<usize, CarrierError> {
        let definitions = sources
            .into_iter()
            .map(SpawnDefinition::from_activation)
            .collect::<Result<Vec<_>, _>>()?;
        self.begin_spawn_definitions(definitions)
    }

    /// SPAWN-1b (Codex P1 on #1981): [`Self::begin_activation_spawns`] for sources whose points
    /// carry their own creature, respawn delay and initial health. Each source stays one
    /// runtime source (one `CREATUREAI0-RL-02` count, point ordinals in authored order); the
    /// ceilings, slot headroom and windowed realization are the same.
    pub fn begin_activation_spawn_sources(
        &mut self,
        sources: Vec<ActivationSpawnSourceFacts>,
    ) -> Result<usize, CarrierError> {
        if sources.len() > CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
        let definitions = sources
            .into_iter()
            .map(SpawnDefinition::from_activation_source)
            .collect::<Result<Vec<_>, _>>()?;
        self.begin_spawn_definitions(definitions)
    }

    fn begin_spawn_definitions(
        &mut self,
        definitions: Vec<SpawnDefinition>,
    ) -> Result<usize, CarrierError> {
        let total_points = self.check_spawn_activation(&definitions)?;
        self.spawn_activation = Some(Box::new(SpawnActivation::new(definitions, total_points)?));
        Ok(total_points)
    }

    /// SPAWN-CEILINGS-1: realizes the next window of the activation begun by
    /// [`Self::begin_activation_spawns`] at owner semantic time `now_micros`: the next
    /// [`spawn::REALIZATIONS_PER_WINDOW_MAX`] points in canonical order, at most one window per
    /// owner window ([`spawn::OWNER_WINDOW_MICROS`]). A second call in an owner
    /// window that already realized one realizes nothing and reports `realized: 0`; the caller
    /// calls again in a later owner window. The activation ends when `remaining` is 0.
    ///
    /// A refusal in any window abandons the activation and realizes nothing: the creatures of
    /// the earlier windows are removed (their slots stay vacant at a later generation), and a
    /// new activation may begin. With no activation in progress it refuses `PlanConflict`.
    pub fn realize_activation_spawn_window(
        &mut self,
        now_micros: u64,
    ) -> Result<SpawnActivationProgress, CarrierError> {
        self.owner_fence()?;
        let mut activation = self
            .spawn_activation
            .take()
            .ok_or(CarrierError::PlanConflict)?;
        let window = now_micros / spawn::OWNER_WINDOW_MICROS;
        if activation.last_window.is_some_and(|last| window <= last) {
            let progress = activation.progress(0);
            self.spawn_activation = Some(activation);
            return Ok(progress);
        }
        match self.realize_activation_window(&mut activation) {
            Ok(realized) => {
                activation.last_window = Some(window);
                let progress = activation.progress(realized);
                if activation.remaining_points > 0 {
                    self.spawn_activation = Some(activation);
                }
                Ok(progress)
            }
            Err(error) => {
                // Every creature the activation admitted is recorded in a realized source (a
                // partly realized one included); they go with their sources. Removal relinks
                // the free list itself, so other owner work between windows is never undone.
                // One pass over the occupied index for all of them (Codex 4242270322).
                let spawns = std::mem::take(&mut self.carrier.spawns);
                self.carrier.remove_all(
                    &self.continuity,
                    spawns
                        .iter()
                        .flat_map(|spawn| spawn.cells.iter().filter_map(|cell| cell.live))
                        .map(|actor| actor.0),
                );
                Err(error)
            }
        }
    }

    /// The checks every activation passes before anything is kept or admitted: the owner
    /// fence, no realized or pending activation, every registered ceiling and the slot
    /// capacity, which keeps [`SPAWN_ACTIVATION_PLAYER_HEADROOM`] slots free for player
    /// admission. Returns the total points.
    fn check_spawn_activation(
        &self,
        definitions: &[SpawnDefinition],
    ) -> Result<usize, CarrierError> {
        self.owner_fence()?;
        if !self.carrier.spawns.is_empty() || self.spawn_activation.is_some() {
            return Err(CarrierError::DuplicateSpawnSource);
        }
        // Every ceiling and the slot capacity are checked once, before anything is admitted,
        // so the work is linear in the points, not in sources × slots.
        let total_points = spawn_points_within_ceilings(definitions)?;
        let context = self.pinned_position_context();
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.carrier.validate_position_context(context)?;
        let headroom = spawn_activation_player_headroom(self.carrier.slots.len());
        if self
            .carrier
            .free_slot_count()
            .checked_sub(total_points)
            .is_none_or(|left| left < headroom)
        {
            return Err(CarrierError::CapacityExceeded);
        }
        Ok(total_points)
    }

    /// Realizes the next owner window of `activation`'s plan: each point in canonical order
    /// admits its creature (a source is recorded at its point 0, so a source may span two
    /// windows). The window's creatures are then bound to their Creature policies. Returns the
    /// creatures realized; on a refusal the caller undoes the activation.
    fn realize_activation_window(
        &mut self,
        activation: &mut SpawnActivation,
    ) -> Result<usize, CarrierError> {
        let context = self.pinned_position_context();
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.carrier.validate_position_context(context)?;
        let Some(window) = activation.windows.get_mut(activation.next_window) else {
            return Ok(0);
        };
        let points = std::mem::take(window);
        debug_assert!(points.len() <= spawn::REALIZATIONS_PER_WINDOW_MAX);
        for point in &points {
            let definition = if point.ordinal == 0 {
                activation
                    .definitions
                    .get_mut(usize::from(point.source))
                    .and_then(Option::take)
            } else {
                None
            };
            self.carrier
                .admit_spawn_point(&self.continuity, *point, definition, context)?;
        }
        activation.next_window += 1;
        activation.realized_points += points.len();
        activation.remaining_points -= points.len();
        for point in &points {
            if let Ok(state) = self
                .carrier
                .spawn_cell_state(SpawnSourceId(point.source), usize::from(point.ordinal))
                && let Some(actor) = state.live
            {
                self.bind_spawned_creature_policy(actor);
            }
        }
        Ok(points.len())
    }

    /// Binds a realized creature to its active Creature policy when the table names its
    /// definition and agrees on its health. Content that names no policy leaves it unbound.
    fn bind_spawned_creature_policy(&mut self, actor: ExactActorRef) {
        let Ok(index) = self.carrier.validate_ref(&self.continuity, actor.0) else {
            return;
        };
        let Slot::CreatureOccupied {
            target_identity, ..
        } = &self.carrier.slots[index]
        else {
            return;
        };
        let Ok(key) = std::str::from_utf8(target_identity).map(str::to_owned) else {
            return;
        };
        if self.companion_policy(&key).is_ok() {
            let _ = self.install_creature_policy(actor, &key);
        }
    }

    /// Kills a live creature in place, as a lethal hit would, for owner-turn spawn tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn kill_creature_for_test(&mut self, actor: ExactActorRef) -> bool {
        let Ok(index) = self.carrier.validate_ref(&self.continuity, actor.0) else {
            return false;
        };
        let Slot::CreatureOccupied { health, .. } = &mut self.carrier.slots[index] else {
            return false;
        };
        *health = 0;
        true
    }

    /// The live creature of each realized spawn point, in canonical point order (the realized
    /// sources are kept ascending and their cells in ordinal order, so no sort is needed).
    pub fn spawn_point_creatures(
        &self,
    ) -> Vec<(
        spawn::SpawnPoint,
        MovementLocalPosition,
        Option<ExactActorRef>,
    )> {
        let mut points = Vec::new();
        for spawn in &self.carrier.spawns {
            for (index, (cell, state)) in spawn
                .definition
                .placement_cells
                .iter()
                .zip(&spawn.cells)
                .enumerate()
            {
                let Ok(ordinal) = u8::try_from(index) else {
                    continue;
                };
                let live = state
                    .live
                    .filter(|actor| self.carrier.creature_alive(&self.continuity, *actor));
                points.push((
                    spawn::SpawnPoint {
                        source: spawn.source.0,
                        ordinal,
                    },
                    MovementLocalPosition {
                        x: cell.x,
                        y: cell.y,
                        floor: cell.floor,
                    },
                    live,
                ));
            }
        }
        debug_assert!(points.windows(2).all(|pair| pair[0].0 < pair[1].0));
        points
    }

    /// The pending respawn occurrence of `point`, if any.
    pub fn pending_respawn(&self, point: spawn::SpawnPoint) -> Option<spawn::PendingRespawn> {
        self.spawn_schedule.pending(point)
    }

    /// Test only: the spawn records read by the drives since the last call (gone-scan points
    /// and deadline entries), and whether the deadline index still equals the schedule.
    #[cfg(any(test, feature = "test-support"))]
    pub fn take_spawn_drive_visits(&mut self) -> (usize, bool) {
        let consistent = self.spawn_deadlines.len() == self.spawn_schedule.len()
            && self.spawn_deadlines.iter().all(|&(due_micros, point)| {
                self.spawn_schedule
                    .pending(point)
                    .is_some_and(|pending| pending.due_micros == due_micros)
            });
        (std::mem::take(&mut self.spawn_drive_visits), consistent)
    }

    /// SPAWN-1a (CREATURE-AI-0 §6.3): one respawn drive at owner time `now_micros`, run from the
    /// Channel owner's creature pass. A point whose creature is gone (observed at the first
    /// drive of an owner window) gets its one occurrence one full delay later; due occurrences
    /// then run by deadline and point: blocked by a player's interest area (blockable points
    /// only), else the warning and admission 4,200 ms later; an occupied cell runs the Occupied
    /// chain. Nothing runs while an activation is in progress.
    ///
    /// SPAWN-CEILINGS-1: the owner drives this once per connected session, so the gone scan is
    /// coalesced per Channel: it runs on the first drive of each owner window
    /// ([`spawn::OWNER_WINDOW_MICROS`]). A later drive of the same window costs only the
    /// occurrences whose deadline it reaches, each still run at its exact deadline.
    ///
    /// SPAWN-CEILINGS-1 (Codex 4242270322): one window runs at most
    /// [`SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX`] steps, so a mass death costs each window a bounded
    /// amount of work; the rest stay pending and run first in the following windows, late by
    /// the windows they wait (their successors are timed from the step that runs). A step reads
    /// the present players once per drive (only their positions, from the occupied index),
    /// checks the cell once and admits without snapshotting the slots
    /// ([`ChannelActorCarrier::respawn_into_free_cell`]).
    ///
    /// SPAWN-CEILINGS-1 (Codex 4243397472): the due occurrences are read from the deadline
    /// index (`spawn_deadlines`) one at a time, at most the window's steps, and the gone scan
    /// reads at most [`SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX`] points from its cursor, so a
    /// window reads at most 4,096 points and 32 index entries however large the backlog: with
    /// 131,072 occurrences due, no window collects or sorts them.
    ///
    /// `sees(observer, cell)` is the reference view (MOVE-RL-11) the owner supplies.
    pub fn drive_spawn_respawns(
        &mut self,
        now_micros: u64,
        sees: impl Fn(MovementLocalPosition, MovementLocalPosition) -> bool,
    ) -> Result<SpawnDriveReport, CarrierError> {
        use spawn::{RespawnOutcome, RespawnStep};
        self.owner_fence()?;
        let mut report = SpawnDriveReport::default();
        if self.carrier.spawns.is_empty() || self.spawn_activation.is_some() {
            return Ok(report);
        }
        let window = now_micros / spawn::OWNER_WINDOW_MICROS;
        if self
            .spawn_drive_window
            .as_ref()
            .is_none_or(|current| window > current.window)
        {
            self.scan_spawn_points(now_micros)?;
            self.spawn_drive_window = Some(SpawnDriveWindow {
                window,
                steps_left: SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX,
            });
        }
        let context = self.pinned_position_context();
        // The present players' positions, read on the first blockable step of this drive.
        let mut players: Option<Vec<MovementLocalPosition>> = None;
        // The window's step budget is checked first, so a drive past it reads no entry.
        while self
            .spawn_drive_window
            .as_ref()
            .is_some_and(|drive_window| drive_window.steps_left > 0)
        {
            let Some(&(due_micros, point)) = self.spawn_deadlines.first() else {
                break;
            };
            #[cfg(any(test, feature = "test-support"))]
            {
                self.spawn_drive_visits += 1;
            }
            if due_micros > now_micros {
                break;
            }
            let Some(pending) = self
                .spawn_schedule
                .pending(point)
                .filter(|pending| pending.due_micros == due_micros)
            else {
                // Not reached: the index is written with every schedule write. An entry the
                // schedule no longer holds would be dropped once.
                self.spawn_deadlines.pop_first();
                continue;
            };
            let timing = self.spawn_timing(point)?;
            let cell = self.spawn_cell(point)?;
            let outcome = match pending.step {
                RespawnStep::Due | RespawnStep::Retry => {
                    let blocked = timing.blockable && pending.step == RespawnStep::Due && {
                        let present = match players.take() {
                            Some(present) => present,
                            None => self.present_player_positions()?,
                        };
                        let blocked = present
                            .iter()
                            .any(|position| position.floor == cell.floor && sees(*position, cell));
                        players = Some(present);
                        blocked
                    };
                    self.spawn_schedule
                        .run_warning_step(point, timing, blocked, now_micros)
                }
                RespawnStep::Admit => {
                    let local = LocalPosition {
                        x: cell.x,
                        y: cell.y,
                        floor: cell.floor,
                    };
                    let occupied = self.carrier.cell_occupied(context, local);
                    if !occupied {
                        let actor = match self.carrier.respawn_into_free_cell(
                            &self.continuity,
                            SpawnSourceId(point.source),
                            usize::from(point.ordinal),
                            context,
                        ) {
                            Ok(actor) => actor,
                            Err(error) => {
                                // Codex 4242715016: a refused admission costs its step, so the
                                // drives of one window retry it at most `SPAWNCEIL-RL-01` times.
                                if let Some(drive_window) = self.spawn_drive_window.as_mut() {
                                    drive_window.steps_left -= 1;
                                }
                                return Err(error);
                            }
                        };
                        self.bind_spawned_creature_policy(actor);
                        report.admitted.push(actor);
                    }
                    self.spawn_schedule
                        .run_admit_step(point, timing, occupied, now_micros)
                }
            }
            .map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
            if let Some(drive_window) = self.spawn_drive_window.as_mut() {
                drive_window.steps_left -= 1;
            }
            // The index follows the step: an admitted occurrence leaves it, any other moves to
            // its next deadline (a successor due within this window runs in it, in order).
            self.spawn_deadlines.remove(&(due_micros, point));
            if let Some(next) = self.spawn_schedule.pending(point) {
                self.spawn_deadlines.insert((next.due_micros, point));
            }
            match outcome {
                RespawnOutcome::Warned { .. } => report.warnings.push(cell),
                RespawnOutcome::Blocked { .. } => report.blocked += 1,
                RespawnOutcome::Retrying { .. } => report.retried += 1,
                RespawnOutcome::Skipped { .. } => report.skipped += 1,
                RespawnOutcome::Admitted => {}
            }
        }
        Ok(report)
    }

    /// Gone detection, once per owner window: reads at most
    /// [`SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX`] realized points in place, from the cursor in
    /// canonical order (sources ascending, cells by ordinal, wrapping) and never one twice, with
    /// no allocation or sort and one generation-exact liveness read per point. A gone creature's
    /// point without an occurrence gets one, one full delay after `now_micros`, and enters the
    /// deadline index. The cursor resumes at the first point at or after where it stopped, so a
    /// source added or removed meanwhile neither repeats nor skips the others.
    fn scan_spawn_points(&mut self, now_micros: u64) -> Result<(), CarrierError> {
        let spawns = &self.carrier.spawns;
        let Some(last_source) = spawns.len().checked_sub(1) else {
            return Ok(());
        };
        let cursor = self.spawn_scan_cursor;
        let mut source = spawns.partition_point(|spawn| spawn.source.0 < cursor.source);
        let mut cell = match spawns.get(source) {
            Some(spawn) if spawn.source.0 == cursor.source => usize::from(cursor.ordinal),
            _ => 0,
        };
        if source > last_source {
            (source, cell) = (0, 0);
        }
        if cell >= spawns[source].cells.len() {
            (source, cell) = (if source == last_source { 0 } else { source + 1 }, 0);
        }
        let start = (source, cell);
        let mut read = 0_usize;
        while read < SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX {
            let spawn = &spawns[source];
            read += 1;
            #[cfg(any(test, feature = "test-support"))]
            {
                self.spawn_drive_visits += 1;
            }
            let gone = !spawn.cells[cell]
                .live
                .is_some_and(|actor| self.carrier.creature_alive(&self.continuity, actor));
            let point = spawn::SpawnPoint {
                source: spawn.source.0,
                ordinal: u8::try_from(cell).map_err(|_| CarrierError::CapacityExceeded)?,
            };
            if gone && self.spawn_schedule.pending(point).is_none() {
                let due_micros = self
                    .spawn_schedule
                    .creature_gone(point, spawn.definition.timing(cell)?, now_micros)
                    .map_err(|_| CarrierError::CapacityExceeded)?;
                self.spawn_deadlines.insert((due_micros, point));
            }
            cell += 1;
            if cell == spawn.cells.len() {
                (source, cell) = (if source == last_source { 0 } else { source + 1 }, 0);
            }
            if (source, cell) == start {
                break;
            }
        }
        let next = &spawns[source];
        self.spawn_scan_cursor = spawn::SpawnPoint {
            source: next.source.0,
            ordinal: u8::try_from(cell).map_err(|_| CarrierError::CapacityExceeded)?,
        };
        Ok(())
    }

    fn spawn_realization(
        &self,
        point: spawn::SpawnPoint,
    ) -> Result<&SpawnRealization, CarrierError> {
        self.carrier
            .spawn_index(SpawnSourceId(point.source))
            .map(|index| &self.carrier.spawns[index])
            .ok_or(CarrierError::UnknownSpawnSource)
    }

    fn spawn_timing(&self, point: spawn::SpawnPoint) -> Result<spawn::PointTiming, CarrierError> {
        self.spawn_realization(point)?
            .definition
            .timing(usize::from(point.ordinal))
    }

    fn spawn_cell(&self, point: spawn::SpawnPoint) -> Result<MovementLocalPosition, CarrierError> {
        let cell = self
            .spawn_realization(point)?
            .definition
            .placement_cells
            .get(usize::from(point.ordinal))
            .ok_or(CarrierError::UnknownSpawnCell)?;
        Ok(MovementLocalPosition {
            x: cell.x,
            y: cell.y,
            floor: cell.floor,
        })
    }

    /// §6.3 Blocked reads the present players of [`Self::positioned_actor_census`] (committed,
    /// positioned and in control); their interest area (MOVE-RL-11) is the caller's view
    /// around these positions. SPAWN-CEILINGS-1 (Codex 4242270322): they are read from the
    /// occupied index without building the census of every creature; a player under a foreign
    /// position context refuses, as in the census.
    fn present_player_positions(&self) -> Result<Vec<MovementLocalPosition>, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.owner_fence()?;
        let context = self.pinned_position_context();
        let mut players = Vec::new();
        for index in &self.carrier.occupied {
            let Some(Slot::Occupied {
                game_session_id: Some(_),
                committed: true,
                lifecycle,
                position: Some(version),
                ..
            }) = usize::try_from(*index)
                .ok()
                .and_then(|index| self.carrier.slots.get(index))
            else {
                continue;
            };
            if lifecycle.control_loss.is_some() {
                continue;
            }
            if version.context != context {
                return Err(CarrierError::PositionContextMismatch);
            }
            players.push(MovementLocalPosition {
                x: version.position.x,
                y: version.position.y,
                floor: version.position.floor,
            });
        }
        Ok(players)
    }

    /// Test only: one live creature at `position`, under the same synthetic context as
    /// [`Self::initialize_movement_test_position`]. Spawn realization is AI-2's carrier path.
    #[cfg(any(test, feature = "test-support"))]
    pub fn admit_test_creature(
        &mut self,
        position: MovementLocalPosition,
    ) -> Result<ExactActorRef, CarrierError> {
        self.admit_monster_lab_creature(position, "test:creature", 20)
    }

    /// Test-only parameterized admission for the native monster laboratory.
    #[cfg(any(test, feature = "test-support"))]
    pub fn admit_monster_lab_creature(
        &mut self,
        position: MovementLocalPosition,
        creature_key: &str,
        health: i64,
    ) -> Result<ExactActorRef, CarrierError> {
        let context = self.test_position_context();
        let actor =
            self.carrier
                .admit_creature(&self.continuity, ActorState(0), creature_key, health)?;
        self.carrier.initialize_position(
            &self.continuity,
            actor,
            context,
            LocalPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
        )?;
        Ok(ExactActorRef(actor))
    }

    /// Test only: actual fixed-slot creature admission with the active pin's position
    /// context, for consumers that independently require qualified current policy reads.
    #[cfg(any(test, feature = "test-support"))]
    pub fn admit_pinned_test_creature(
        &mut self,
        position: MovementLocalPosition,
    ) -> Result<ExactActorRef, CarrierError> {
        let context = self.pinned_position_context();
        let actor =
            self.carrier
                .admit_creature(&self.continuity, ActorState(0), "test:creature", 20)?;
        self.carrier.initialize_position(
            &self.continuity,
            actor,
            context,
            LocalPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
        )?;
        Ok(ExactActorRef(actor))
    }

    /// Test only: despawn one actor, so its generation goes stale.
    #[cfg(any(test, feature = "test-support"))]
    pub fn remove_test_actor(&mut self, actor: ExactActorRef) -> Result<(), CarrierError> {
        self.carrier.remove(&self.continuity, actor.0).map(|_| ())
    }

    pub fn reserve_fresh_session(
        &mut self,
        game_session_id: GameSessionId,
    ) -> Result<PlayerActorReservation, CarrierError> {
        self.carrier
            .reserve_player(&self.continuity, game_session_id)
    }

    pub fn commit_fresh_session(
        &mut self,
        reservation: PlayerActorReservation,
    ) -> Result<ExactActorRef, CarrierError> {
        self.carrier
            .commit_reserved_player(&self.continuity, reservation)
    }

    pub fn rollback_definitely_uncommitted(
        &mut self,
        reservation: PlayerActorReservation,
    ) -> Result<(), CarrierError> {
        self.carrier
            .rollback_reserved_player(&self.continuity, reservation)
    }

    /// Exact-ref cleanup only. Callers still need an authoritative terminal
    /// GameSession fact; ordinary socket loss is not such a fact.
    pub fn remove_terminal_session(
        &mut self,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        self.carrier
            .remove_terminal_player(&self.continuity, game_session_id, actor)
    }

    /// A2: bind the admitted session's Character lease to its committed player slot. Binding the
    /// identical lease again is a no-op; a different one conflicts.
    pub fn bind_attacker_lease(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        lease: CharacterLease,
    ) -> Result<(), CarrierError> {
        self.carrier
            .bind_attacker_lease(&self.continuity, actor.0, game_session_id, lease)
    }

    /// The grace-expiry token of a control-loss epoch.
    pub const fn grace_expiry_fence(epoch: u64) -> WriteFenceToken {
        WriteFenceToken::ControlLoss(epoch)
    }

    /// The token of a transition id from [`Self::mint_transition_fence`].
    pub const fn transition_fence(id: u64) -> WriteFenceToken {
        WriteFenceToken::Transition(id)
    }

    /// A fresh transition id for a transition other than grace expiry.
    pub fn mint_transition_fence(&mut self) -> Result<u64, CarrierError> {
        let next = self
            .carrier
            .fence_transitions
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        self.carrier.fence_transitions = next;
        Ok(next)
    }

    /// CHARM-DESC-FENCE-LEASE step (a): fence the slot's damage writes for `game_session_id`,
    /// before the durable transition. A grace-expiry token needs the slot's control-loss mark of
    /// that epoch. One transition per session: another token is `WriteFenceBusy`.
    pub fn fence_player_writes(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        token: WriteFenceToken,
    ) -> Result<WriteFenceSet, CarrierError> {
        self.carrier
            .fence_player_writes(&self.continuity, actor.0, game_session_id, token)
    }

    /// Step (c) for a durable outcome that left the session holding the lease: lift the fence
    /// with its exact token. Any other token is a no-op; returns whether the fence was lifted.
    pub fn lift_player_fence(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        token: WriteFenceToken,
    ) -> Result<bool, CarrierError> {
        self.carrier
            .lift_player_fence(&self.continuity, actor.0, game_session_id, token)
    }

    /// D295 item 4: the post-grace in-place rebind. One operation detaches the terminal session,
    /// drops its fence and control-loss mark, and binds the newly authorized session and its
    /// newer lease of the same Character. The actor reference is unchanged.
    #[allow(
        dead_code,
        reason = "the post-grace takeover is not composed into the transport yet"
    )]
    pub fn rebind_player_session(
        &mut self,
        actor: ExactActorRef,
        terminal: GameSessionId,
        successor: GameSessionId,
        lease: CharacterLease,
    ) -> Result<(), CarrierError> {
        self.carrier
            .rebind_player_session(&self.continuity, actor.0, terminal, successor, lease)
    }

    /// FND-04B §22: bind a reconstructed slot of a same-session continuation only after its
    /// durable reconcile. Terminal or unprovable refuses and leaves the slot unbound, so no
    /// damage write is admitted for the session.
    #[allow(
        dead_code,
        reason = "process-replacement continuation is not composed yet; the carrier gate is"
    )]
    pub fn bind_continuation(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        reconcile: ContinuationReconcile,
    ) -> Result<(), CarrierError> {
        self.carrier
            .bind_continuation(&self.continuity, actor.0, game_session_id, reconcile)
    }

    /// Test-only census: (committed player actors, pending player reservations).
    #[cfg(any(test, feature = "test-support"))]
    pub fn player_slot_counts(&self) -> (usize, usize) {
        self.carrier
            .slots
            .iter()
            .fold((0, 0), |(committed, pending), slot| match slot {
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    ..
                } => (committed + 1, pending),
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: false,
                    ..
                } => (committed, pending + 1),
                _ => (committed, pending),
            })
    }

    /// Test-only census: the position revisions of committed player actors that
    /// stand at the pinned start cell under the pinned context, ascending.
    #[cfg(any(test, feature = "test-support"))]
    pub fn entry_start_player_revisions(&self) -> Vec<u64> {
        let mut revisions: Vec<u64> = self
            .carrier
            .slots
            .iter()
            .filter_map(|slot| match slot {
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    position: Some(version),
                    ..
                } if version.position == self.content.entry_start
                    && version.context == self.pinned_position_context() =>
                {
                    Some(version.revision)
                }
                _ => None,
            })
            .collect();
        revisions.sort_unstable();
        revisions
    }

    /// Test-only census: the recorded control-loss epochs of committed players, ascending.
    #[cfg(any(test, feature = "test-support"))]
    pub fn player_control_loss_epochs(&self) -> Vec<u64> {
        let mut epochs: Vec<u64> = self
            .carrier
            .slots
            .iter()
            .filter_map(|slot| match slot {
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    lifecycle,
                    ..
                } => lifecycle.control_loss.map(|mark| mark.epoch),
                _ => None,
            })
            .collect();
        epochs.sort_unstable();
        epochs
    }

    /// Test-only census: committed player actors positioned at the pinned
    /// start cell under the pinned context by one initialization (revision 1).
    #[cfg(any(test, feature = "test-support"))]
    pub fn players_positioned_at_entry_start(&self) -> usize {
        self.carrier
            .slots
            .iter()
            .filter(|slot| {
                matches!(slot, Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    position: Some(version),
                    ..
                } if version.position == self.content.entry_start
                    && version.revision == 1
                    && version.context == self.pinned_position_context())
            })
            .count()
    }

    #[cfg(any(test, feature = "test-support"))]
    fn contains_committed_session(
        &self,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> bool {
        self.carrier
            .contains_committed_player(&self.continuity, game_session_id, actor)
    }
}

impl ChannelActorCarrier {
    pub fn current_owner_combat_death<'a>(
        &'a mut self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerCombatDeath<'a> {
        CurrentOwnerCombatDeath {
            carrier: self,
            continuity,
        }
    }

    fn current_owner_movement_position<'a>(
        &'a mut self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerMovementPosition<'a> {
        CurrentOwnerMovementPosition {
            carrier: self,
            continuity,
        }
    }

    /// GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1 §4.5 ("Creature slots become
    /// Movement-capable actors of the same Movement owner"): a creature slot with a live
    /// (`health > 0`) generation reads exactly like a player slot here. `validate_ref` already
    /// rejects a stale generation, a vacant slot or an exhausted one before this is reached, and
    /// `read_position`/`compare_commit_position` already branch on `Slot::CreatureOccupied`
    /// identically to `Slot::Occupied` for position facts. NPC-BEHAVIOUR-0 §4.4 also admits
    /// the existing NPC slot to this Movement read; the separate combat/AI position read
    /// continues to exclude NPCs.
    fn read_movement_position(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        if let Slot::NpcOccupied {
            generation,
            position,
            ..
        } = &self.slots[index]
        {
            if *generation != actor_ref.actor_local_generation.0 {
                return Err(CarrierError::StaleActorGeneration);
            }
            if position.actor_local_id != actor_ref.actor_local_id
                || position.actor_local_generation != actor_ref.actor_local_generation
            {
                return Err(CarrierError::PositionSnapshotMismatch);
            }
            return Ok(MovementPositionSnapshot(PositionSnapshot {
                actor_ref,
                version: *position,
            }));
        }
        self.read_position(continuity, actor_ref)
            .map(MovementPositionSnapshot)
    }

    /// See [`Self::read_movement_position`]: a creature slot commits its cardinal step through
    /// the same compare-commit path a player's does (§4.5). The NPC branch writes that
    /// actor's existing position only after current-owner, exact-snapshot and cardinal
    /// checks. It does not grant NPCs the spell relocation or combat position paths.
    fn commit_movement_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        expected: PositionSnapshot,
        next: LocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, expected.actor_ref)?;
        if let Slot::NpcOccupied {
            generation,
            position,
            ..
        } = &self.slots[index]
        {
            self.validate_position_context(expected.version.context)?;
            if *generation != expected.actor_ref.actor_local_generation.0
                || position.actor_local_id != expected.actor_ref.actor_local_id
                || position.actor_local_generation != expected.actor_ref.actor_local_generation
                || *position != expected.version
            {
                return Err(CarrierError::PositionSnapshotMismatch);
            }
            let facing = match (
                next.floor == position.position.floor,
                next.x.checked_sub(position.position.x),
                next.y.checked_sub(position.position.y),
            ) {
                (true, Some(1), Some(0)) => MovementFacing::East,
                (true, Some(-1), Some(0)) => MovementFacing::West,
                (true, Some(0), Some(1)) => MovementFacing::South,
                (true, Some(0), Some(-1)) => MovementFacing::North,
                _ => return Err(CarrierError::MovementNonCardinal),
            };
            let version = VersionedPosition {
                position: next,
                revision: position
                    .revision
                    .checked_add(1)
                    .ok_or(CarrierError::PositionRevisionExhausted)?,
                facing: Some(facing),
                ..*position
            };
            let Slot::NpcOccupied { position, .. } = &mut self.slots[index] else {
                unreachable!("exclusively borrowed NPC slot remains current");
            };
            *position = version;
            return Ok(MovementPositionSnapshot(PositionSnapshot {
                actor_ref: expected.actor_ref,
                version,
            }));
        }
        // An exclusive carrier borrow prevents a slot replacement between this check and
        // the existing private exact-snapshot/revision compare-commit.
        self.compare_commit_position(continuity, expected, expected.version.context, next)
            .map(MovementPositionSnapshot)
    }

    pub fn current_owner_exact_commit<'a>(
        &'a mut self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerExactActorCommit<'a> {
        self.current_owner_exact_commit_at(continuity, None)
    }

    pub fn current_owner_exact_commit_at<'a>(
        &'a mut self,
        continuity: &'a NamespaceContinuityGuard,
        now_ms: Option<u64>,
    ) -> CurrentOwnerExactActorCommit<'a> {
        CurrentOwnerExactActorCommit {
            content: None,
            carrier: self,
            continuity,
            now_ms,
        }
    }

    pub fn current_owner_exact_lookup<'a>(
        &'a self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerExactActorLookup<'a> {
        CurrentOwnerExactActorLookup {
            carrier: self,
            continuity,
        }
    }

    pub fn bootstrap_pre_production(
        continuity: &mut NamespaceContinuityGuard,
        explicit_capacity: usize,
    ) -> Result<Self, CarrierError> {
        if explicit_capacity == 0 {
            return Err(CarrierError::InvalidCapacity);
        }

        // Validate every identity/byte calculation before allocation or publication.
        u32::try_from(explicit_capacity).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        explicit_capacity
            .checked_mul(size_of::<Slot>())
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;

        // Reject replay before touching allocation while retaining the claim
        // commit until every fallible construction step has succeeded.
        continuity.ensure_current_generation_unclaimed()?;
        let slots = allocate_slots(explicit_capacity)?;
        let mut occupied = Vec::new();
        occupied
            .try_reserve_exact(explicit_capacity)
            .map_err(|_| CarrierError::AllocationFailed)?;

        // Claim only after every fallible construction step has succeeded.
        continuity.claim_current_generation()?;

        Ok(Self {
            world_id: continuity.world_id,
            channel_id: continuity.channel_id,
            scope_generation: continuity.current_generation,
            slots: slots.into_boxed_slice(),
            free_head: Some(0),
            corpse_projections: Vec::new(),
            death_reward_occurrences: Vec::new(),
            spawns: Vec::new(),
            native_summons: NativeSummonLinks::default(),
            native_summon_deaths: [None; 64],
            native_summon_death_cursor: 0,
            bone_shared_groups: Vec::new(),
            fence_transitions: 0,
            attackers: Vec::new(),
            occupied,
            corpse_item_bindings: Vec::new(),
            corpse_projection_pins: Vec::new(),
        })
    }

    pub fn admit(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorState,
    ) -> Result<ActorRef, CarrierError> {
        self.admit_inner(continuity, actor, None, true, None, false)
    }

    /// AI-2 (GAME-AI-01 §4.1: "the carrier's current one-creature limit ... is lifted to this
    /// envelope in AI-2"): more than one creature actor may be admitted per Channel
    /// generation. Its capacity gates are `CREATUREAI0-RL-04`
    /// ([`CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX`]) and the general slot-capacity check
    /// `admit_inner` performs; population/placement-cell ceilings are a spawn-realization
    /// concern (`realize_spawn`), not this generic per-actor primitive.
    pub fn admit_creature(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorState,
        target_identity: &str,
        initial_health: i64,
    ) -> Result<ActorRef, CarrierError> {
        if initial_health <= 0 {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        self.validate_current_continuity(continuity)?;
        // `CREATUREAI0-RL-04`, before anything is selected or written.
        if self.occupied.len() >= CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
        let target_identity = creature_target_binding(target_identity)?;
        self.admit_inner(
            continuity,
            actor,
            None,
            true,
            Some((initial_health, target_identity)),
            false,
        )
    }

    pub fn reserve_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        game_session_id: GameSessionId,
    ) -> Result<PlayerActorReservation, CarrierError> {
        let actor_ref = self.admit_inner(
            continuity,
            ActorState(0),
            Some(game_session_id),
            false,
            None,
            false,
        )?;
        Ok(PlayerActorReservation {
            game_session_id,
            actor_ref,
        })
    }

    pub fn commit_reserved_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        reservation: PlayerActorReservation,
    ) -> Result<ExactActorRef, CarrierError> {
        let index = self.validate_ref(continuity, reservation.actor_ref)?;
        // SPELL-LOCK-2 §1.3: a slot reserved by a pending spell batch refuses retryably.
        self.assert_slot_spell_unreserved(index)?;
        match &mut self.slots[index] {
            Slot::Occupied {
                generation,
                game_session_id: Some(game_session_id),
                committed,
                ..
            } if *generation == reservation.actor_ref.actor_local_generation.0
                && *game_session_id == reservation.game_session_id
                && !*committed =>
            {
                *committed = true;
                Ok(ExactActorRef(reservation.actor_ref))
            }
            _ => Err(CarrierError::PlayerReservationMismatch),
        }
    }

    fn rollback_reserved_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        reservation: PlayerActorReservation,
    ) -> Result<(), CarrierError> {
        let index = self.validate_ref(continuity, reservation.actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                game_session_id: Some(game_session_id),
                committed,
                ..
            } if *generation == reservation.actor_ref.actor_local_generation.0
                && *game_session_id == reservation.game_session_id
                && !*committed => {}
            _ => return Err(CarrierError::PlayerReservationMismatch),
        }
        self.remove(continuity, reservation.actor_ref).map(|_| ())
    }

    pub fn remove_terminal_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        let index = self.validate_ref(continuity, actor.0)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                game_session_id: Some(stored_session),
                committed,
                ..
            } if *generation == actor.0.actor_local_generation.0
                && *stored_session == game_session_id
                && *committed => {}
            _ => return Err(CarrierError::PlayerReservationMismatch),
        }
        self.remove(continuity, actor.0).map(|_| ())
    }

    fn contains_committed_player(
        &self,
        continuity: &NamespaceContinuityGuard,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> bool {
        let Ok(index) = self.validate_ref(continuity, actor.0) else {
            return false;
        };
        matches!(
            &self.slots[index],
            Slot::Occupied {
                generation,
                game_session_id: Some(stored_session),
                committed: true,
                ..
            } if *generation == actor.0.actor_local_generation.0
                && *stored_session == game_session_id
        )
    }

    fn admit_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorState,
        game_session_id: Option<GameSessionId>,
        committed: bool,
        initial_health: Option<(i64, Arc<[u8]>)>,
        fail_after_selection: bool,
    ) -> Result<ActorRef, CarrierError> {
        // Current outer authority must be proven before slot selection or any
        // mutation, including terminal generation-exhaustion bookkeeping.
        self.validate_current_continuity(continuity)?;
        let Some(free_head) = self.free_head else {
            return Err(CarrierError::CapacityExceeded);
        };
        let index =
            usize::try_from(free_head).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;

        let Slot::VacantReusable {
            generation,
            next_free,
        } = &self.slots[index]
        else {
            unreachable!("free-list head must name a reusable slot");
        };
        let generation = *generation;
        let next_free = *next_free;
        let Some(next_generation) = generation.checked_add(1) else {
            // Protected #541 exception: this is the sole failure that mutates state.
            self.slots[index] = Slot::Exhausted { generation };
            self.free_head = next_free;
            return Err(CarrierError::ActorGenerationExhausted);
        };
        let actor_local_id = index
            .checked_add(1)
            .and_then(|value| u32::try_from(value).ok())
            .map(ActorLocalId)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let actor_ref = ActorRef {
            world_id: self.world_id,
            channel_id: self.channel_id,
            scope_generation: self.scope_generation,
            actor_local_id,
            actor_local_generation: ActorLocalGeneration(next_generation),
        };

        if fail_after_selection {
            return Err(CarrierError::InjectedAdmissionFailure);
        }
        self.slots[index] = if let Some((health, target_identity)) = initial_health {
            Slot::CreatureOccupied {
                generation: next_generation,
                actor,
                position: None,
                target_identity,
                health,
                committed: Box::default(),
                damage_contributors: Box::default(),
                companion: None,
                spell_combat: Box::new(runtime_actor_spell::ActorCombatState::creature(health)),
            }
        } else {
            Slot::Occupied {
                generation: next_generation,
                actor,
                game_session_id,
                committed,
                position: None,
                lifecycle: Box::default(),
                spell_combat: Box::default(),
            }
        };
        self.free_head = next_free;
        // Within the capacity reserved at bootstrap: one entry per occupied slot.
        self.occupied.push(free_head);
        Ok(actor_ref)
    }

    pub fn lookup(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<&ActorState, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                actor,
                committed: true,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => Ok(actor),
            Slot::CreatureOccupied {
                generation,
                actor,
                health,
                ..
            } if *generation == actor_ref.actor_local_generation.0 && *health > 0 => Ok(actor),
            Slot::CreatureOccupied {
                generation, health, ..
            } if *generation == actor_ref.actor_local_generation.0 && *health == 0 => {
                Err(CarrierError::CreatureNotActionable)
            }
            Slot::NpcOccupied { generation, .. }
                if *generation == actor_ref.actor_local_generation.0 =>
            {
                Err(CarrierError::NotCreature)
            }
            Slot::Occupied { .. }
            | Slot::NpcOccupied { .. }
            | Slot::CreatureOccupied { .. }
            | Slot::CreatureReserved { .. }
            | Slot::VacantReusable { .. }
            | Slot::Exhausted { .. } => Err(CarrierError::StaleActorGeneration),
        }
    }

    fn assert_slot_spell_unreserved(&self, index: usize) -> Result<(), CarrierError> {
        match &self.slots[index] {
            Slot::Occupied { spell_combat, .. } | Slot::CreatureOccupied { spell_combat, .. }
                if spell_combat.pending_owner.is_none()
                    && spell_combat.pending_source.is_none() =>
            {
                Ok(())
            }
            _ => Err(CarrierError::PlanConflict),
        }
    }

    /// Drops `index` from the occupied-slot index when its slot returns to the free list.
    fn unindex_occupied(&mut self, index: u32) {
        if let Some(at) = self.occupied.iter().position(|entry| *entry == index) {
            self.occupied.swap_remove(at);
        }
    }

    /// The borrowing check of [`Self::remove`] (ARCH-SPELL-LOCK-2 §1.6): it changes nothing.
    /// `released` names slots whose owner reservation the same owner turn releases first.
    pub(super) fn check_remove(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        released: &[usize],
    ) -> Result<(), CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                spell_combat,
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                spell_combat,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                if spell_combat.pending_source.is_some()
                    || (spell_combat.pending_owner.is_some() && !released.contains(&index))
                {
                    return Err(CarrierError::PlanConflict);
                }
            }
            Slot::Occupied { .. } | Slot::CreatureOccupied { .. } => {
                return Err(CarrierError::StaleActorGeneration);
            }
            Slot::CreatureReserved { .. }
            | Slot::NpcOccupied { .. }
            | Slot::VacantReusable { .. }
            | Slot::Exhausted { .. } => {
                return Err(CarrierError::PlanConflict);
            }
        }
        for (child_count, link) in self
            .native_summons
            .children(ExactActorRef(actor_ref))
            .enumerate()
        {
            if child_count == 16 {
                return Err(CarrierError::CapacityExceeded);
            }
            self.validate_summon_actor(continuity, link.child)?;
        }
        for link in self.native_summons.children(ExactActorRef(actor_ref)) {
            self.check_remove(continuity, link.child.0, released)?;
        }
        u32::try_from(index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        Ok(())
    }

    pub fn remove(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<ActorState, CarrierError> {
        let (actor, index) = self.remove_keeping_indexes(continuity, actor_ref)?;
        self.unindex_slot(index);
        Ok(actor)
    }

    /// Drops the vacated slot `index` from the occupied index and its attacker lease.
    fn unindex_slot(&mut self, index: usize) {
        if let Ok(free_index) = u32::try_from(index) {
            self.unindex_occupied(free_index);
        }
        // A2: the slot's bound lease and fence go with it.
        self.attackers.retain(|entry| entry.index != index);
    }

    /// SPAWN-CEILINGS-1 (Codex 4242270322): removes every actor of `actors` (best effort, as
    /// [`Self::remove`] each), then drops their vacated slots from the occupied index and the
    /// attacker leases in one pass each, so removing `n` actors costs `n` plus one pass over
    /// each list instead of a pass per actor. Returns the actors removed.
    fn remove_all(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actors: impl IntoIterator<Item = ActorRef>,
    ) -> usize {
        let mut vacated = Vec::new();
        if vacated.try_reserve_exact(self.slots.len()).is_err() {
            return actors
                .into_iter()
                .filter(|actor| self.remove(continuity, *actor).is_ok())
                .count();
        }
        vacated.resize(self.slots.len(), false);
        let mut removed = 0;
        for actor in actors {
            if let Ok((_, index)) = self.remove_keeping_indexes(continuity, actor) {
                vacated[index] = true;
                removed += 1;
            }
        }
        self.occupied.retain(|index| {
            !usize::try_from(*index).is_ok_and(|index| vacated.get(index) == Some(&true))
        });
        self.attackers
            .retain(|entry| vacated.get(entry.index) != Some(&true));
        removed
    }

    /// [`Self::remove`] except the occupied index and the attacker leases, which the caller
    /// updates for the returned vacated slot.
    fn remove_keeping_indexes(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<(ActorState, usize), CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        self.assert_slot_spell_unreserved(index)?;
        let (generation, actor) = match &self.slots[index] {
            Slot::Occupied {
                generation, actor, ..
            }
            | Slot::CreatureOccupied {
                generation, actor, ..
            } if *generation == actor_ref.actor_local_generation.0 => (*generation, *actor),
            Slot::NpcOccupied { .. } => return Err(CarrierError::PlanConflict),
            Slot::Occupied { .. }
            | Slot::CreatureOccupied { .. }
            | Slot::CreatureReserved { .. }
            | Slot::VacantReusable { .. }
            | Slot::Exhausted { .. } => return Err(CarrierError::StaleActorGeneration),
        };
        // Parent despawn removes children, without death/corpse/rewards. The relation never
        // admits child masters, so this bounded cleanup cannot recurse into another family.
        let mut children = [None; 16];
        for (child_count, link) in self
            .native_summons
            .children(ExactActorRef(actor_ref))
            .enumerate()
        {
            if child_count == 16 {
                return Err(CarrierError::CapacityExceeded);
            }
            self.validate_summon_actor(continuity, link.child)?;
            children[child_count] = Some(link.child);
        }
        for child in children.into_iter().flatten() {
            self.remove(continuity, child.0)?;
        }
        if let Some(death) = self
            .native_summons
            .child(ExactActorRef(actor_ref))
            .and_then(|link| link.death)
            && !self
                .native_summon_deaths
                .iter()
                .flatten()
                .any(|d| d.child == death.child)
        {
            self.native_summon_deaths[self.native_summon_death_cursor] = Some(death);
            self.native_summon_death_cursor = (self.native_summon_death_cursor + 1) % 64;
        }
        self.native_summons.remove_child(ExactActorRef(actor_ref));
        let free_index =
            u32::try_from(index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        self.slots[index] = Slot::VacantReusable {
            generation,
            next_free: self.free_head,
        };
        self.free_head = Some(free_index);
        // A retained corpse projection / reward occurrence deliberately outlives this slot
        // (existing Combat D1/D2 behavior: a lost-response retry must still reconcile after
        // administrative removal, `projection_failures_preserve_retry_and_lost_response_idempotency`).
        // `project_committed_lethal_inner`/`reward_occurrence_inner` bound those collections
        // with FIFO eviction instead. A spawn cell that names this creature is not scanned for
        // here (SPAWN-CEILINGS-1: that would cost every removal O(spawn points)); every reader
        // of `SpawnCellState::live` checks the exact generation (`creature_alive`, `remove`),
        // so the stale reference reads as gone.
        Ok((actor, index))
    }

    /// D140-D142: up to [`COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX`] receipts are
    /// retained in the creature's own slot. An identical replay of a retained occurrence returns
    /// the recorded transition without mutation. A new distinct occurrence against a live
    /// creature either applies (evicting the oldest evictable receipt when the list is full) or
    /// is refused as stale / over capacity. All validation, checked arithmetic and fallible
    /// allocation precede the first mutation. Administrative removal drops the receipts with the
    /// slot and emits no death or corpse event.
    ///
    /// An attributed commit (`attacker` is `Some`) is identified solely by the carrier-derived
    /// `(CharacterId, GameSessionId, sequence, sub_ordinal)`; `command.occurrence` is not read.
    /// An unattributed commit keeps the caller's opaque `occurrence` identity and is never
    /// evictable.
    // Structural sibling supplies no current content authority; registered encounter actors refuse.
    pub fn commit_creature_damage_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.commit_creature_damage_with_pin_inner(
            continuity,
            actor,
            command,
            attacker,
            fail_before_write,
            None,
            Some(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
            None,
        )
    }

    /// `max_sub_ordinal` and `now_ms` as in [`Self::commit_creature_damage_inner_limited`];
    /// `current_pin` revalidates registered shared-HP encounter actors.
    #[allow(clippy::too_many_arguments)]
    fn commit_creature_damage_with_pin_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
        current_pin: Option<&ChannelContentPin>,
        max_sub_ordinal: Option<u16>,
        now_ms: Option<u64>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        if let Some(g) = self.bone_shared_groups.iter().find(|g| {
            g.phylactery == ExactActorRef(actor_ref) || g.cages.contains(&ExactActorRef(actor_ref))
        }) {
            let pin = current_pin.ok_or(CarrierError::PositionContextMismatch)?;
            if pin.world_id != g.world
                || pin.activation_sequence != g.activation
                || pin.server_artifact_digest != g.server
                || pin.client_artifact_digest != g.client
                || pin.frame_binding_digest != g.frame
                || pin.map_revision_digest != g.map
            {
                return Err(CarrierError::PositionContextMismatch);
            }
            if self.world_id != g.world
                || self.channel_id != g.channel
                || self.scope_generation != g.generation
            {
                return Err(CarrierError::WrongScope);
            }
        }
        if let Some(g) = self
            .bone_shared_groups
            .iter()
            .find(|g| g.phylactery == ExactActorRef(actor_ref))
            && g.cage_deaths.iter().any(Option::is_none)
        {
            return Err(CarrierError::CreatureNotActionable);
        }
        if let Some(g) = self
            .bone_shared_groups
            .iter()
            .find(|g| g.cages.contains(&ExactActorRef(actor_ref)))
            .cloned()
        {
            return self.commit_bone_shared_damage_inner(
                continuity,
                &g,
                actor_ref,
                command,
                attacker,
                fail_before_write,
                max_sub_ordinal,
                now_ms,
            );
        }
        let (index, staged, result) = self.prepare_creature_damage_inner_bounded(
            continuity,
            actor_ref,
            command,
            attacker,
            fail_before_write,
            max_sub_ordinal,
            false,
            now_ms,
        )?;
        if let Some(staged) = staged {
            self.slots[index] = staged;
        }
        Ok(result)
    }

    fn prepare_creature_damage_inner(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
    ) -> Result<(usize, Option<Slot>, OwnerDamageResult), CarrierError> {
        self.prepare_creature_damage_inner_bounded(
            continuity,
            actor_ref,
            command,
            attacker,
            fail_before_write,
            Some(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
            false,
            None,
        )
    }

    /// ATTACK-1b: refuses a swing ordinal at or below the attacker's mark for the same lineage
    /// (`lease`, session and sequence) unless its exact receipt is retained, which the commit
    /// then replays idempotently.
    fn swing_lineage_admission(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        attacker: AttackerCommand,
    ) -> Result<(), CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        let Slot::CreatureOccupied {
            committed,
            damage_contributors,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if committed
            .entries
            .iter()
            .any(|record| record.origin == Some(attacker.origin()))
        {
            return Ok(());
        }
        match damage_contributors.high_water(attacker.character) {
            Some((lease, session, sequence, sub_ordinal))
                if lease == attacker.lease_generation
                    && session == attacker.session
                    && sequence == attacker.sequence
                    && attacker.sub_ordinal <= sub_ordinal =>
            {
                Err(CarrierError::StaleAttackerSequence)
            }
            _ => Ok(()),
        }
    }

    // Preserve main95 source batch limits/deferred high-water semantics. This entry
    // has no current content pin, so registered shared-HP encounter actors refuse.
    #[allow(clippy::too_many_arguments)]
    fn commit_creature_damage_inner_bounded(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
        max_sub_ordinal: u16,
        deferred: bool,
        now_ms: Option<u64>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.commit_creature_damage_inner_limited(
            continuity,
            actor_ref,
            command,
            attacker,
            fail_before_write,
            Some(max_sub_ordinal),
            deferred,
            now_ms,
        )
    }

    /// `max_sub_ordinal` is the exclusive bound on an attributed `sub_ordinal`; `None` admits
    /// the whole `u16` range (ATTACK-1b swing ordinals). `now_ms` is the committing owner
    /// turn's clock, recorded on the receipt.
    #[allow(clippy::too_many_arguments)]
    fn commit_creature_damage_inner_limited(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
        max_sub_ordinal: Option<u16>,
        deferred: bool,
        now_ms: Option<u64>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        if self.bone_shared_groups.iter().any(|g| {
            g.phylactery == ExactActorRef(actor_ref) || g.cages.contains(&ExactActorRef(actor_ref))
        }) {
            return Err(CarrierError::PositionContextMismatch);
        }
        let (index, staged, result) = self.prepare_creature_damage_inner_bounded(
            continuity,
            actor_ref,
            command,
            attacker,
            fail_before_write,
            max_sub_ordinal,
            deferred,
            now_ms,
        )?;
        if let Some(staged) = staged {
            self.slots[index] = staged;
        }
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_creature_damage_inner_bounded(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
        max_sub_ordinal: Option<u16>,
        deferred: bool,
        now_ms: Option<u64>,
    ) -> Result<(usize, Option<Slot>, OwnerDamageResult), CarrierError> {
        let OwnerDamageCommand {
            target,
            occurrence,
            binding,
            damage,
        } = command;
        if let (Some(attacker), Some(max)) = (attacker, max_sub_ordinal)
            && attacker.sub_ordinal >= max
        {
            return Err(CarrierError::SubOrdinalOutOfRange);
        }
        let index = self.validate_ref(continuity, actor_ref)?;
        self.assert_slot_spell_unreserved(index)?;
        if binding.is_empty()
            || attacker.is_some_and(|attacker| attacker.lease_generation == 0)
            || (attacker.is_none() && (occurrence.is_empty() || occurrence.contains(&0)))
        {
            // An unsequenced occurrence containing the NUL delimiter would make its identity
            // ambiguous against `binding.split(0)`, so it is rejected here.
            return Err(CarrierError::InvalidCommitBinding);
        }
        if binding.len() > MAX_OWNER_COMMIT_BINDING_BYTES
            || (attacker.is_none() && occurrence.len() > MAX_OWNER_COMMIT_BINDING_BYTES)
        {
            return Err(CarrierError::CommitBindingTooLarge);
        }
        if attacker.is_none()
            && (!binding.starts_with(occurrence) || binding.get(occurrence.len()) != Some(&0))
        {
            return Err(CarrierError::InvalidCommitBinding);
        }
        if damage <= 0 {
            return Err(CarrierError::InvalidDamage);
        }
        let Slot::CreatureOccupied {
            generation,
            target_identity,
            health,
            committed,
            damage_contributors,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        if target_identity.as_ref() != target {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let prior = committed
            .entries
            .iter()
            .find(|record| match (attacker, record.origin) {
                (Some(attacker), Some(origin)) => attacker.origin() == origin,
                (None, None) => record.binding.split(|byte| *byte == 0).next() == Some(occurrence),
                _ => false,
            });
        if let Some(prior) = prior {
            if prior.binding.as_ref() != binding || prior.damage != damage {
                return Err(CarrierError::PlanConflict);
            }
            return Ok((
                index,
                None,
                OwnerDamageResult {
                    applied: false,
                    ..prior.result
                },
            ));
        }
        if *health == 0 {
            return Err(CarrierError::CreatureNotActionable);
        }
        if let Some(attacker) = attacker {
            if deferred {
                if let Some((lease, session, _, _)) =
                    damage_contributors.high_water(attacker.character)
                    && (attacker.lease_generation < lease
                        || (attacker.lease_generation == lease && attacker.session != session))
                {
                    return Err(CarrierError::SupersededAttackerSession);
                }
            } else {
                damage_contributors.admission(attacker)?;
            }
        }
        let read_len = committed.entries.len();
        let evict = if read_len >= COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX {
            Some(
                committed
                    .oldest_evictable(damage_contributors)
                    .ok_or(CarrierError::DamageReceiptCapacityExceeded)?,
            )
        } else {
            None
        };
        // D142: the owner damage-application ordinal, assigned to distinct occurrences only.
        let ordinal = committed.next_ordinal;
        let next_ordinal = ordinal
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let next = health
            .checked_sub(damage)
            .ok_or(CarrierError::DamageOverflow)?
            .max(0);
        let result = OwnerDamageResult {
            applied: true,
            health_before: *health,
            health_after: next,
        };
        let binding = copy_bounded_binding(binding)?;
        let receipt = OwnerCommitRecord {
            binding,
            damage,
            result,
            ordinal,
            origin: attacker.map(AttackerCommand::origin),
        };
        if fail_before_write {
            return Err(CarrierError::InjectedCommitFailure);
        }
        // Recheck current authority and local generation at the mutation
        // boundary, independent of Ability's earlier resolution snapshot.
        let write_index = self.validate_ref(continuity, actor_ref)?;
        if write_index != index {
            return Err(CarrierError::StaleActorGeneration);
        }
        let mut staged = self.slots[index].clone();
        let Slot::CreatureOccupied {
            generation,
            target_identity,
            health,
            committed,
            damage_contributors,
            companion,
            ..
        } = &mut staged
        else {
            return Err(CarrierError::StaleActorGeneration);
        };
        if *generation != actor_ref.actor_local_generation.0
            || target_identity.as_ref() != target
            || *health != result.health_before
            || committed.next_ordinal != ordinal
            || committed.entries.len() != read_len
        {
            return Err(CarrierError::StaleActorGeneration);
        }
        // Every fallible allocation happens before the first mutation below.
        if evict.is_none() {
            committed
                .entries
                .try_reserve(1)
                .map_err(|_| CarrierError::CommitAllocationFailed)?;
        }
        if attacker.is_some()
            && damage_contributors.entries.len() < COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX
        {
            damage_contributors
                .entries
                .try_reserve(1)
                .map_err(|_| CarrierError::CommitAllocationFailed)?;
        }
        if let Some(evicted) = evict {
            committed.entries.remove(evicted);
        }
        committed.entries.push(receipt);
        committed.next_ordinal = next_ordinal;
        if next == 0 && *health > 0 {
            committed.lethal_at_ms = now_ms;
        }
        *health = next;
        // Frozen source Monster::drainHealth removes invisibility on an applied
        // health drain. This is the target's actual condition owner; players
        // retain their invisibility and historical replay returns above.
        if let Some(companion) = companion {
            companion
                .conditions
                .remove_type(super::condition::ConditionType::Invisible);
        }
        if next == 0 {
            committed.conditions.die();
        }
        // D132/D3-3: attribute this applied hit to its attacker only now, at the sole mutation
        // boundary, after every replay/staleness check above -- an idempotent replay of a
        // retained occurrence returns earlier and never reaches here, so it can never
        // double-attribute. D141 sets the attacker's high-water mark from the same commit.
        if let Some(attacker) = attacker {
            // Codex P1: credit the HP actually removed, not the requested/planned damage -- an
            // overkill hit (e.g. 100 damage on 1 remaining HP) must only ever credit the 1 HP
            // that could actually be removed (`next <= health` always, since `damage > 0`).
            let removed = result.health_before.saturating_sub(result.health_after);
            damage_contributors.record(
                attacker.character,
                u64::try_from(removed).unwrap_or(u64::MAX),
                ordinal,
                Some(match damage_contributors.high_water(attacker.character) {
                    Some(mark)
                        if deferred
                            && mark.0 == attacker.lease_generation
                            && mark.1 == attacker.session
                            && (mark.2, mark.3) > (attacker.sequence, attacker.sub_ordinal) =>
                    {
                        mark
                    }
                    _ => attacker.high_water(),
                }),
                now_ms,
            );
        }
        Ok((index, Some(staged), result))
    }

    /// D132/D3-3: read-only, non-mutating lookup of `actor`'s current
    /// `DamageContributors::top_damage_character()` in this generation's still-live slot. See
    /// [`CurrentOwnerCombatDeath::top_damage_character`].
    fn top_damage_character_inner(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<Option<CharacterId>, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        let Slot::CreatureOccupied {
            generation,
            damage_contributors,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        Ok(damage_contributors.top_damage_character())
    }

    /// The winner's identity is the lease generation and session of its D141 mark, and its
    /// slot is the committed player slot bound to exactly that lease and session. A winner
    /// without a mark, or whose slot is gone or rebound to a successor, is `Gone`: nothing
    /// looks it up by `CharacterId` alone.
    fn top_damage_contributor_inner(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<TopDamagePrincipal, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        let Slot::CreatureOccupied {
            generation,
            damage_contributors,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        let Some(winner) = damage_contributors.top_contributor() else {
            return Ok(TopDamagePrincipal::Untracked);
        };
        let Some((lease_generation, session, _, _)) = winner.high_water else {
            return Ok(TopDamagePrincipal::Gone);
        };
        let slot = self.attackers.iter().find_map(|entry| {
            let lease = entry.authority.lease?;
            if lease.character_id() != winner.character || lease.generation() != lease_generation {
                return None;
            }
            match self.slots.get(entry.index)? {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(bound),
                    committed: true,
                    ..
                } if *generation == entry.generation && *bound == session => {
                    Some((entry.index, *generation))
                }
                _ => None,
            }
        });
        let Some((slot_index, slot_generation)) = slot else {
            return Ok(TopDamagePrincipal::Gone);
        };
        Ok(TopDamagePrincipal::Present(TopDamageContributor {
            character: winner.character,
            lease_generation,
            session,
            actor: ExactActorRef(ActorRef {
                world_id: self.world_id,
                channel_id: self.channel_id,
                scope_generation: self.scope_generation,
                actor_local_id: ActorLocalId(
                    u32::try_from(slot_index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                actor_local_generation: ActorLocalGeneration(slot_generation),
            }),
            last_damage_at_ms: winner.last_damage_at_ms,
        }))
    }

    fn committed_lethal_receipt_inner(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<CommittedLethalReceipt, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        if let Some(existing) = self
            .corpse_projections
            .iter()
            .find(|projection| projection.occurrence.actor == ExactActorRef(actor_ref))
        {
            return Ok(CommittedLethalReceipt {
                projection: RuntimeCorpseProjection {
                    occurrence: CreatureDeathOccurrenceRef {
                        actor: existing.occurrence.actor,
                        commit_binding: copy_bounded_binding(&existing.occurrence.commit_binding)?,
                        damage: existing.occurrence.damage,
                        health_before: existing.occurrence.health_before,
                    },
                    position: existing.position,
                    death_at_ms: existing.death_at_ms,
                },
            });
        }
        let Slot::CreatureOccupied {
            generation,
            position,
            health,
            committed,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::CommittedLethalUnavailable);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        let death_at_ms = committed.lethal_at_ms;
        // D144: the unique retained receipt whose commit drove health to zero.
        let Some(committed) = committed.lethal().filter(|_| *health == 0) else {
            return Err(CarrierError::CommittedLethalUnavailable);
        };
        let position = position.ok_or(CarrierError::PositionUnavailable)?;
        Ok(CommittedLethalReceipt {
            projection: RuntimeCorpseProjection {
                occurrence: CreatureDeathOccurrenceRef {
                    actor: ExactActorRef(actor_ref),
                    commit_binding: copy_bounded_binding(&committed.binding)?,
                    damage: committed.damage,
                    health_before: committed.result.health_before,
                },
                position,
                death_at_ms,
            },
        })
    }

    fn validate_lethal_receipt(
        &self,
        continuity: &NamespaceContinuityGuard,
        receipt: &CommittedLethalReceipt,
    ) -> Result<(), CarrierError> {
        let projection = &receipt.projection;
        let actor_ref = projection.occurrence.actor.0;
        let index = self.validate_ref(continuity, actor_ref)?;
        let Slot::CreatureOccupied {
            generation,
            position: Some(position),
            health,
            committed,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::StaleActorGeneration);
        };
        if *generation != actor_ref.actor_local_generation.0 || committed.entries.is_empty() {
            return Err(CarrierError::StaleActorGeneration);
        }
        // D144: the unique retained receipt whose commit drove health to zero.
        let Some(committed) = committed.lethal() else {
            return Err(CarrierError::CorpseReceiptMismatch);
        };
        if *health != 0
            || committed.binding.as_ref() != projection.occurrence.commit_binding.as_ref()
            || committed.damage != projection.occurrence.damage
            || committed.result.health_before != projection.occurrence.health_before
            || *position != projection.position
        {
            return Err(CarrierError::CorpseReceiptMismatch);
        }
        Ok(())
    }

    /// AI-2: per actor now, not a channel-wide singleton (§4.1 envelope). A different actor's
    /// already-projected death never blocks or is disturbed by this call.
    fn project_committed_lethal_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        receipt: CommittedLethalReceipt,
        fail_before_write: bool,
        fail_after_write: bool,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        let actor = receipt.projection.occurrence.actor;
        self.validate_ref(continuity, actor.0)?;
        if self.is_native_summon(actor) {
            return Err(CarrierError::SummonHasNoRewards);
        }
        if self
            .corpse_projections
            .iter()
            .any(|projection| projection.occurrence.actor == actor)
        {
            return self.existing_corpse_projection(&receipt);
        }
        self.validate_summon_actor(continuity, actor)?;
        self.validate_lethal_receipt(continuity, &receipt)?;
        if fail_before_write {
            return Err(CarrierError::InjectedCorpseProjectionFailure);
        }
        self.cascade_native_summon_deaths(continuity, actor)?;
        let projected = receipt.projection;
        let captured_sealed_key = projected.occurrence.death_key();
        // Bounded FIFO retention (`MAX_RETAINED_CORPSE_PROJECTIONS`): a retained projection
        // deliberately outlives its actor's slot (lost-response replay, above), so this Vec is
        // not otherwise pruned; evict the oldest unpinned entry rather than grow without bound
        // (a pinned one is an accepted kill settlement's, Codex 4248192466). The
        // evicted death's memoized reward occurrence goes with it: a reward occurrence never
        // outlives, and is never evicted independently of, its projection, so a retained
        // projection can never re-mint a second occurrence for the same death.
        self.retain_corpse_projection(projected);
        self.note_bone_shared_projection(actor, captured_sealed_key);
        if fail_after_write {
            return Err(CarrierError::InjectedCorpseResponseFailure);
        }
        self.corpse_projections
            .last()
            .ok_or(CarrierError::CorpseProjectionConflict)
    }

    fn existing_corpse_projection(
        &self,
        receipt: &CommittedLethalReceipt,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        let actor = receipt.projection.occurrence.actor;
        let existing = self
            .corpse_projections
            .iter()
            .find(|projection| projection.occurrence.actor == actor)
            .ok_or(CarrierError::CorpseProjectionConflict)?;
        if existing == &receipt.projection {
            Ok(existing)
        } else {
            Err(CarrierError::CorpseProjectionConflict)
        }
    }

    /// Codex 4248192466: the eviction takes the oldest unpinned projection, so a projection that
    /// an accepted kill settlement still binds to survives any number of newer deaths (a full
    /// kill queue's, a familiar's, a crystal's). At most `MAX_RETAINED_CORPSE_PROJECTIONS`
    /// unpinned and `MAX_PINNED_CORPSE_PROJECTIONS` pinned projections are retained.
    fn retain_corpse_projection(&mut self, projected: RuntimeCorpseProjection) {
        while self.unpinned_corpse_projections() >= MAX_RETAINED_CORPSE_PROJECTIONS {
            let pins = &self.corpse_projection_pins;
            let Some(index) = self
                .corpse_projections
                .iter()
                .position(|projection| !pins.contains(&projection.occurrence.actor))
            else {
                break;
            };
            let evicted = self.corpse_projections.remove(index).occurrence.actor.0;
            self.death_reward_occurrences
                .retain(|reward| reward.actor != evicted);
        }
        self.corpse_projections.push(projected);
    }

    fn unpinned_corpse_projections(&self) -> usize {
        self.corpse_projections
            .iter()
            .filter(|projection| {
                !self
                    .corpse_projection_pins
                    .contains(&projection.occurrence.actor)
            })
            .count()
    }

    /// Whether `actor`'s committed death is projected and retained in this world, Channel and
    /// scope generation.
    fn retains_corpse_projection(&self, actor: ExactActorRef) -> bool {
        actor.world_id() == self.world_id
            && actor.channel_id() == self.channel_id
            && actor.scope_generation() == self.scope_generation
            && self
                .corpse_projections
                .iter()
                .any(|projection| projection.occurrence.actor == actor)
    }

    fn pin_corpse_projection_inner(&mut self, actor: ExactActorRef) -> Result<(), CarrierError> {
        if !self.retains_corpse_projection(actor) {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        if self.corpse_projection_pins.contains(&actor) {
            return Ok(());
        }
        if self.corpse_projection_pins.len() >= MAX_PINNED_CORPSE_PROJECTIONS {
            return Err(CarrierError::CapacityExceeded);
        }
        self.corpse_projection_pins
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        self.corpse_projection_pins.push(actor);
        Ok(())
    }

    fn unpin_corpse_projection_inner(&mut self, actor: ExactActorRef) {
        self.corpse_projection_pins
            .retain(|pinned| *pinned != actor);
    }

    /// D2b: the memoized XP reward occurrence of this generation's committed
    /// death (DUR-03 decision §4.2). The projected corpse must already name
    /// `actor_ref`. A first call mints and stores the occurrence; every later
    /// call for the same `(actor, character)` returns the identical stored
    /// value with no new mint; a different `character` for the same actor
    /// conflicts (`RewardPrincipalConflict`) rather than silently reassigning
    /// the reward.
    /// Returns the occurrence bytes and whether this exact call minted them
    /// (`true`) or reused an already-memoized value (`false`). The caller
    /// uses that distinction to decide whether progression initialization is
    /// needed again: on a fresh mint (a first attempt this generation) it is;
    /// on reuse (a retry) the R7 P03 XP writer's own occurrence-keyed replay
    /// resolves without it.
    fn reward_occurrence_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        character: [u8; 16],
    ) -> Result<([u8; 16], bool), CarrierError> {
        self.validate_ref(continuity, actor_ref)?;
        if self.is_native_summon(ExactActorRef(actor_ref)) {
            return Err(CarrierError::SummonHasNoRewards);
        }
        let committed = self
            .corpse_projections
            .iter()
            .any(|projection| projection.occurrence.actor == ExactActorRef(actor_ref));
        if !committed {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        if let Some(existing) = self
            .death_reward_occurrences
            .iter()
            .find(|entry| entry.actor == actor_ref)
        {
            return if existing.character == character {
                Ok((existing.occurrence, false))
            } else {
                Err(CarrierError::RewardPrincipalConflict)
            };
        }
        let occurrence = mint_reward_occurrence_bytes(actor_ref, character);
        // Bounded by `corpse_projections`: at most one entry per retained projection.
        self.death_reward_occurrences.push(DeathRewardOccurrence {
            actor: actor_ref,
            character,
            occurrence,
        });
        Ok((occurrence, true))
    }

    fn validate_position_context(
        &self,
        context: PreProductionPositionContext,
    ) -> Result<(), CarrierError> {
        if context.world_id != self.world_id
            || context.channel_id != self.channel_id
            || context.scope_generation != self.scope_generation
            || context.coordinate_frame_marker == 0
            || context.map_revision_marker == 0
            || context.content_generation_marker == 0
        {
            return Err(CarrierError::InvalidPreProductionPositionContext);
        }
        Ok(())
    }

    /// Private preproduction-only initial binding; existing actor admission
    /// cannot silently invent a position or a Content activation context.
    pub fn initialize_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        context: PreProductionPositionContext,
        position: LocalPosition,
    ) -> Result<PositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        self.assert_slot_spell_unreserved(index)?;
        self.validate_position_context(context)?;
        match &mut self.slots[index] {
            Slot::CreatureOccupied {
                generation,
                health: 0,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                Err(CarrierError::CreatureNotActionable)
            }
            Slot::Occupied {
                generation,
                committed: true,
                position: stored @ None,
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: stored @ None,
                health: 1..,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                let version = VersionedPosition {
                    actor_local_id: actor_ref.actor_local_id,
                    actor_local_generation: actor_ref.actor_local_generation,
                    context,
                    position,
                    revision: 1,
                    facing: None,
                };
                *stored = Some(version);
                Ok(PositionSnapshot { actor_ref, version })
            }
            Slot::Occupied {
                generation,
                committed: true,
                position: Some(_),
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: Some(_),
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                Err(CarrierError::PositionAlreadyInitialized)
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }

    /// One direct occupied-slot read under the independently current owner.
    pub fn read_position(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<PositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                committed: true,
                position: Some(version),
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: Some(version),
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                if version.actor_local_id != actor_ref.actor_local_id
                    || version.actor_local_generation != actor_ref.actor_local_generation
                {
                    return Err(CarrierError::PositionSnapshotMismatch);
                }
                Ok(PositionSnapshot {
                    actor_ref,
                    version: *version,
                })
            }
            Slot::Occupied {
                generation,
                committed: true,
                position: None,
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: None,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                Err(CarrierError::PositionUnavailable)
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }

    /// Compare and commit exactly one replacement in the same actor slot.
    /// All fallible checks run before the sole mutation and no alternate store
    /// or movement timing authority participates.
    pub fn compare_commit_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        expected: PositionSnapshot,
        next_context: PreProductionPositionContext,
        next_position: LocalPosition,
    ) -> Result<PositionSnapshot, CarrierError> {
        self.compare_commit_position_with_summon_lifecycle(
            continuity,
            expected,
            next_context,
            next_position,
            false,
        )
    }
    fn compare_commit_position_with_summon_lifecycle(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        expected: PositionSnapshot,
        next_context: PreProductionPositionContext,
        next_position: LocalPosition,
        allow_self_removal: bool,
    ) -> Result<PositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, expected.actor_ref)?;
        self.assert_slot_spell_unreserved(index)?;
        if matches!(
            &self.slots[index],
            Slot::CreatureOccupied {
                generation,
                health: 0,
                ..
            } if *generation == expected.actor_ref.actor_local_generation.0
        ) {
            return Err(CarrierError::CreatureNotActionable);
        }
        self.validate_position_context(next_context)?;
        if next_context != expected.version.context {
            return Err(CarrierError::PositionContextMismatch);
        }
        let (generation, current) = match &self.slots[index] {
            Slot::Occupied {
                generation,
                committed: true,
                position: Some(current),
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: Some(current),
                ..
            } => (*generation, *current),
            _ => return Err(CarrierError::PositionSnapshotMismatch),
        };
        if generation != expected.actor_ref.actor_local_generation.0
            || current.actor_local_id != expected.actor_ref.actor_local_id
            || current.actor_local_generation != expected.actor_ref.actor_local_generation
            || current != expected.version
        {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        let separated_summons =
            self.summons_separated_by_move(continuity, expected.actor_ref, next_position)?;
        if !allow_self_removal
            && separated_summons
                .iter()
                .flatten()
                .any(|child| child.0 == expected.actor_ref)
        {
            return Err(CarrierError::MovementCreatureUnavailable);
        }
        // Restore every store administrative removal touches, including attacker ownership
        // (removal no longer writes the spawn cells, so they need no snapshot).
        let summon_rollback = if separated_summons.iter().any(Option::is_some) {
            Some((
                self.slots.clone(),
                self.free_head,
                self.occupied.clone(),
                self.native_summons.clone(),
                self.native_summon_deaths,
                self.native_summon_death_cursor,
                self.attackers.clone(),
            ))
        } else {
            None
        };
        let revision = current
            .revision
            .checked_add(1)
            .ok_or(CarrierError::PositionRevisionExhausted)?;
        let version = VersionedPosition {
            actor_local_id: expected.actor_ref.actor_local_id,
            actor_local_generation: expected.actor_ref.actor_local_generation,
            context: next_context,
            position: next_position,
            revision,
            facing: match (
                next_position.floor == current.position.floor,
                next_position.x.checked_sub(current.position.x),
                next_position.y.checked_sub(current.position.y),
            ) {
                (true, Some(1), Some(0)) => Some(MovementFacing::East),
                (true, Some(-1), Some(0)) => Some(MovementFacing::West),
                (true, Some(0), Some(1)) => Some(MovementFacing::South),
                (true, Some(0), Some(-1)) => Some(MovementFacing::North),
                _ => current.facing,
            },
        };
        match &mut self.slots[index] {
            Slot::Occupied { position, .. } | Slot::CreatureOccupied { position, .. } => {
                *position = Some(version)
            }
            _ => unreachable!("validated occupied slot"),
        }
        if let Err(error) = self.remove_separated_summons(continuity, separated_summons) {
            if let Some((slots, free, occupied, links, deaths, cursor, attackers)) = summon_rollback
            {
                self.slots = slots;
                self.free_head = free;
                self.occupied = occupied;
                self.native_summons = links;
                self.native_summon_deaths = deaths;
                self.native_summon_death_cursor = cursor;
                self.attackers = attackers;
            }
            return Err(error);
        }
        Ok(PositionSnapshot {
            actor_ref: expected.actor_ref,
            version,
        })
    }

    fn player_slot_index(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
    ) -> Result<usize, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                game_session_id: Some(bound),
                committed: true,
                ..
            } if *generation == actor_ref.actor_local_generation.0 && *bound == game_session_id => {
                Ok(index)
            }
            Slot::Occupied { generation, .. } | Slot::CreatureOccupied { generation, .. }
                if *generation == actor_ref.actor_local_generation.0 =>
            {
                Err(CarrierError::PlayerReservationMismatch)
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }

    pub fn player_control_loss(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
    ) -> Result<Option<ControlLossMark>, CarrierError> {
        let index = self.player_slot_index(continuity, actor_ref, game_session_id)?;
        match &self.slots[index] {
            Slot::Occupied { lifecycle, .. } => Ok(lifecycle.control_loss),
            _ => Err(CarrierError::PlayerReservationMismatch),
        }
    }

    pub fn record_player_control_loss(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
        mark: ControlLossMark,
    ) -> Result<(), CarrierError> {
        if mark.epoch == 0 {
            return Err(CarrierError::ControlLossConflict);
        }
        let index = self.player_slot_index(continuity, actor_ref, game_session_id)?;
        self.assert_slot_spell_unreserved(index)?;
        match &mut self.slots[index] {
            Slot::Occupied { lifecycle, .. } => match lifecycle.control_loss {
                None => {
                    lifecycle.control_loss = Some(mark);
                    Ok(())
                }
                Some(existing) if existing == mark => Ok(()),
                _ => Err(CarrierError::ControlLossConflict),
            },
            _ => Err(CarrierError::ControlLossConflict),
        }
    }

    pub fn restore_player_control(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
        epoch: u64,
    ) -> Result<(), CarrierError> {
        let index = self.player_slot_index(continuity, actor_ref, game_session_id)?;
        self.assert_slot_spell_unreserved(index)?;
        match &mut self.slots[index] {
            Slot::Occupied { lifecycle, .. } => match lifecycle.control_loss {
                Some(mark) if mark.epoch == epoch => {
                    lifecycle.control_loss = None;
                    // CHARM-DESC-FENCE-LEASE §3 item 3: a resume lifts the fence of its epoch.
                    let generation = actor_ref.actor_local_generation.0;
                    if let Some(entry) = self
                        .attackers
                        .iter_mut()
                        .find(|entry| entry.index == index && entry.generation == generation)
                        && entry.authority.fence == Some(WriteFenceToken::ControlLoss(epoch))
                    {
                        entry.authority.fence = None;
                    }
                    Ok(())
                }
                None => Ok(()),
                _ => Err(CarrierError::ControlLossConflict),
            },
            _ => Err(CarrierError::ControlLossConflict),
        }
    }

    fn bound_attacker_lease(
        &self,
        continuity: &NamespaceContinuityGuard,
        attacker: ActorRef,
        game_session_id: GameSessionId,
    ) -> Result<CharacterLease, CarrierError> {
        // The three checks, in order: the slot holds the command's session; it is not fenced;
        // its lease is bound.
        let index =
            self.player_slot_index(continuity, attacker, game_session_id)
                .map_err(|error| match error {
                    CarrierError::PlayerReservationMismatch
                    | CarrierError::StaleActorGeneration => CarrierError::SupersededAttackerSession,
                    other => other,
                })?;
        let authority = self
            .attacker_authority(index, attacker.actor_local_generation.0)
            .unwrap_or_default();
        if authority.fence.is_some() {
            return Err(CarrierError::SupersededAttackerSession);
        }
        authority
            .lease
            .ok_or(CarrierError::SupersededAttackerSession)
    }

    fn attacker_authority(&self, index: usize, generation: u64) -> Option<PlayerAttackerAuthority> {
        self.attackers
            .iter()
            .find(|entry| entry.index == index && entry.generation == generation)
            .map(|entry| entry.authority)
    }

    /// The authority entry of the committed player slot `actor_ref` bound to `game_session_id`,
    /// inserted empty when absent.
    fn attacker_authority_mut(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
    ) -> Result<&mut PlayerAttackerAuthority, CarrierError> {
        let index = self.player_slot_index(continuity, actor_ref, game_session_id)?;
        let generation = actor_ref.actor_local_generation.0;
        let at = match self.attackers.iter().position(|entry| entry.index == index) {
            Some(at) => {
                // A recycled slot's stale entry; `remove` normally prunes it first.
                if self.attackers[at].generation != generation {
                    self.attackers[at] = PlayerAttackerEntry {
                        index,
                        generation,
                        authority: PlayerAttackerAuthority::default(),
                    };
                }
                at
            }
            None => {
                self.attackers
                    .try_reserve(1)
                    .map_err(|_| CarrierError::AllocationFailed)?;
                self.attackers.push(PlayerAttackerEntry {
                    index,
                    generation,
                    authority: PlayerAttackerAuthority::default(),
                });
                self.attackers.len() - 1
            }
        };
        Ok(&mut self.attackers[at].authority)
    }

    pub fn bind_attacker_lease(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
        lease: CharacterLease,
    ) -> Result<(), CarrierError> {
        let authority = self.attacker_authority_mut(continuity, actor_ref, game_session_id)?;
        match authority.lease {
            None => {
                authority.lease = Some(lease);
                Ok(())
            }
            Some(bound) if bound == lease => Ok(()),
            Some(_) => Err(CarrierError::AttackerBindingConflict),
        }
    }

    pub fn fence_player_writes(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
        token: WriteFenceToken,
    ) -> Result<WriteFenceSet, CarrierError> {
        if let WriteFenceToken::ControlLoss(epoch) = token
            && self
                .player_control_loss(continuity, actor_ref, game_session_id)?
                .is_none_or(|mark| mark.epoch != epoch)
        {
            return Err(CarrierError::ControlLossConflict);
        }
        let authority = self.attacker_authority_mut(continuity, actor_ref, game_session_id)?;
        match authority.fence {
            None => {
                authority.fence = Some(token);
                Ok(WriteFenceSet::Fenced)
            }
            Some(set) if set == token => Ok(WriteFenceSet::Joined),
            Some(_) => Err(CarrierError::WriteFenceBusy),
        }
    }

    pub fn lift_player_fence(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
        token: WriteFenceToken,
    ) -> Result<bool, CarrierError> {
        let authority = self.attacker_authority_mut(continuity, actor_ref, game_session_id)?;
        if authority.fence == Some(token) {
            authority.fence = None;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn rebind_player_session(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        terminal: GameSessionId,
        successor: GameSessionId,
        lease: CharacterLease,
    ) -> Result<(), CarrierError> {
        // SPELL-LOCK-2 §1.3: a slot reserved by a pending spell batch keeps its session binding
        // until that batch installs or releases; the rebind refuses retryably before any write.
        let index = self.player_slot_index(continuity, actor_ref, terminal)?;
        self.assert_slot_spell_unreserved(index)?;
        let authority = self.attacker_authority_mut(continuity, actor_ref, terminal)?;
        // Only a newer lease of the same Character, under a different session, is a successor.
        match authority.lease {
            Some(bound)
                if bound.character_id() == lease.character_id()
                    && bound.generation() < lease.generation()
                    && successor != terminal => {}
            _ => return Err(CarrierError::AttackerBindingConflict),
        }
        // One step under the runtime lock: the terminal session's fence goes with its binding.
        *authority = PlayerAttackerAuthority {
            lease: Some(lease),
            fence: None,
        };
        if let Slot::Occupied {
            game_session_id,
            lifecycle,
            ..
        } = &mut self.slots[index]
        {
            *game_session_id = Some(successor);
            lifecycle.control_loss = None;
        }
        Ok(())
    }

    pub fn bind_continuation(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
        reconcile: ContinuationReconcile,
    ) -> Result<(), CarrierError> {
        // Terminal: only the post-grace path may attach control. Unprovable: fail closed. Both
        // leave the slot unbound, so no damage write is admitted for the session.
        let ContinuationReconcile::Proven(lease) = reconcile else {
            return Err(CarrierError::ContinuationRefused);
        };
        let authority = self.attacker_authority_mut(continuity, actor_ref, game_session_id)?;
        // A reconstructed slot carries no fence and no other binding.
        if authority.fence.is_some() || authority.lease.is_some_and(|bound| bound != lease) {
            return Err(CarrierError::ContinuationRefused);
        }
        authority.lease = Some(lease);
        Ok(())
    }

    fn validate_ref(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<usize, CarrierError> {
        self.validate_current_continuity(continuity)?;
        if actor_ref.world_id != continuity.world_id
            || actor_ref.channel_id != continuity.channel_id
            || actor_ref.scope_generation != continuity.current_generation
            || actor_ref.world_id != self.world_id
            || actor_ref.channel_id != self.channel_id
            || actor_ref.scope_generation != self.scope_generation
        {
            return Err(CarrierError::WrongScope);
        }
        let index = actor_ref
            .actor_local_id
            .0
            .checked_sub(1)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(CarrierError::InvalidActorIdentity)?;
        if index >= self.slots.len() || actor_ref.actor_local_generation.0 == 0 {
            return Err(CarrierError::InvalidActorIdentity);
        }
        Ok(index)
    }

    fn validate_current_continuity(
        &self,
        continuity: &NamespaceContinuityGuard,
    ) -> Result<(), CarrierError> {
        if !continuity.active
            || continuity.world_id != self.world_id
            || continuity.channel_id != self.channel_id
            || continuity.current_generation != self.scope_generation
        {
            return Err(CarrierError::WrongScope);
        }
        Ok(())
    }

    // AI-2 (§4.3, §4.9, D116): spawn realization and respawn.

    /// The index of `source` in `self.spawns`, which is kept in strictly ascending source
    /// order: a binary search, not a scan of every realized source.
    fn spawn_index(&self, source: SpawnSourceId) -> Option<usize> {
        self.spawns
            .binary_search_by_key(&source.0, |spawn| spawn.source.0)
            .ok()
    }

    /// Reusable vacant slots: the most creatures this carrier can still admit.
    fn free_slot_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| matches!(slot, Slot::VacantReusable { .. }))
            .count()
    }

    /// §4.3: realizes one spawn source outside an activation batch: admits one creature per
    /// declared placement cell, each with a fresh actor-local generation. Every fallible check
    /// that does not itself mutate `self` (continuity, the position context, duplicate source,
    /// the registered `CREATUREAI0-RL-02` source and `CREATUREAI0-RL-01` point ceilings,
    /// general capacity) precedes any admission. Admission itself is staged: `self.slots`/
    /// `self.free_head`/`self.occupied` are snapshotted first, and a failure partway through
    /// the population (for example `ActorGenerationExhausted`, the one documented exception
    /// that still mutates a single slot on failure, `admit_inner`) rolls the complete
    /// realization back to that snapshot before returning the error, so a retry never
    /// accumulates untracked, unpositioned creatures (GAME-AI-01 §6: over budget means zero
    /// mutation, applied here to any per-creature failure, not only capacity).
    ///
    /// Activation realizes its points through `admit_spawn_point` instead; this per-source
    /// snapshot is for the single-source paths only.
    fn realize_spawn(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        source: SpawnSourceId,
        definition: SpawnDefinition,
        position_context: PreProductionPositionContext,
    ) -> Result<(), CarrierError> {
        self.validate_current_continuity(continuity)?;
        self.validate_position_context(position_context)?;
        if self.spawn_index(source).is_some() {
            return Err(CarrierError::DuplicateSpawnSource);
        }
        if self.spawns.len() >= CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
        let realized_points = self
            .spawns
            .iter()
            .try_fold(0usize, |sum, spawn| sum.checked_add(spawn.cells.len()))
            .and_then(|sum| sum.checked_add(definition.placement_cells.len()))
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        if realized_points > CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
        if self.free_slot_count() < definition.population {
            return Err(CarrierError::CapacityExceeded);
        }
        let slots_before = self.slots.clone();
        let free_head_before = self.free_head;
        let occupied_before = self.occupied.clone();
        if let Err(error) =
            self.admit_spawn_source(continuity, source, definition, position_context)
        {
            self.slots = slots_before;
            self.free_head = free_head_before;
            self.occupied = occupied_before;
            return Err(error);
        }
        Ok(())
    }

    /// Admits and positions one creature per declared cell of `definition` and records the
    /// source at its ordered place in `self.spawns`. It is not staged: the caller has already
    /// checked continuity, the position context, the registered ceilings and slot capacity,
    /// holds the snapshot, and restores it on any error. A duplicate source or a source past
    /// `CREATUREAI0-RL-02` still refuses before any admission.
    fn admit_spawn_source(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        source: SpawnSourceId,
        definition: SpawnDefinition,
        position_context: PreProductionPositionContext,
    ) -> Result<(), CarrierError> {
        let insert_at = match self
            .spawns
            .binary_search_by_key(&source.0, |spawn| spawn.source.0)
        {
            Ok(_) => return Err(CarrierError::DuplicateSpawnSource),
            Err(insert_at) => insert_at,
        };
        if self.spawns.len() >= CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
        let mut cells = Vec::new();
        cells
            .try_reserve_exact(definition.placement_cells.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        self.spawns
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (cell, point) in definition.placement_cells.iter().zip(&definition.points) {
            let actor = self.admit_creature(
                continuity,
                ActorState(0),
                &point.creature_target_identity,
                point.creature_initial_health,
            )?;
            self.initialize_position(continuity, actor, position_context, *cell)?;
            cells.push(SpawnCellState {
                live: Some(ExactActorRef(actor)),
                attempt: 0,
                successor: 0,
            });
        }
        self.spawns.insert(
            insert_at,
            SpawnRealization {
                source,
                definition,
                cells,
            },
        );
        Ok(())
    }

    /// SPAWN-CEILINGS-1 (CREATURE-AI-0 §6.2): admits and positions the creature of one
    /// activation point, in canonical order. Point 0 records its source (with `definition`) at
    /// the source's ordered place in `self.spawns`; every point appends its cell state, so a
    /// source realized across two owner windows stays recorded in between. The caller has
    /// already checked the ceilings and slot capacity. A refusal leaves every creature it
    /// admitted recorded in `self.spawns` for the caller's undo.
    fn admit_spawn_point(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        point: spawn::SpawnPoint,
        definition: Option<SpawnDefinition>,
        position_context: PreProductionPositionContext,
    ) -> Result<(), CarrierError> {
        let source = SpawnSourceId(point.source);
        if point.ordinal == 0 {
            let definition = definition.ok_or(CarrierError::PlanConflict)?;
            let insert_at = match self
                .spawns
                .binary_search_by_key(&source.0, |spawn| spawn.source.0)
            {
                Ok(_) => return Err(CarrierError::DuplicateSpawnSource),
                Err(insert_at) => insert_at,
            };
            if self.spawns.len() >= CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX {
                return Err(CarrierError::CapacityExceeded);
            }
            let mut cells = Vec::new();
            cells
                .try_reserve_exact(definition.placement_cells.len())
                .map_err(|_| CarrierError::AllocationFailed)?;
            self.spawns
                .try_reserve(1)
                .map_err(|_| CarrierError::AllocationFailed)?;
            self.spawns.insert(
                insert_at,
                SpawnRealization {
                    source,
                    definition,
                    cells,
                },
            );
        }
        let index = self
            .spawn_index(source)
            .ok_or(CarrierError::UnknownSpawnSource)?;
        let realization = &self.spawns[index];
        let ordinal = usize::from(point.ordinal);
        if realization.cells.len() != ordinal {
            return Err(CarrierError::PlanConflict);
        }
        let cell = *realization
            .definition
            .placement_cells
            .get(ordinal)
            .ok_or(CarrierError::PlanConflict)?;
        let spawned = realization.definition.point(ordinal)?;
        let health = spawned.creature_initial_health;
        let identity = spawned.creature_target_identity.clone();
        let actor = self.admit_creature(continuity, ActorState(0), &identity, health)?;
        // Reserved exactly at point 0: the push never reallocates.
        self.spawns[index].cells.push(SpawnCellState {
            live: Some(ExactActorRef(actor)),
            attempt: 0,
            successor: 0,
        });
        self.initialize_position(continuity, actor, position_context, cell)?;
        Ok(())
    }

    /// True when some other *live* actor (player or creature, `health > 0`) currently occupies
    /// `cell` under `position_context` (§4.3: "if the cell is occupied when the timer is
    /// due"). A dead creature's own corpse (`health == 0`) never counts.
    fn cell_occupied(
        &self,
        position_context: PreProductionPositionContext,
        cell: LocalPosition,
    ) -> bool {
        self.slots.iter().any(|slot| match slot {
            Slot::Occupied {
                committed: true,
                position: Some(version),
                ..
            }
            | Slot::CreatureOccupied {
                position: Some(version),
                health: 1..,
                ..
            } => version.context == position_context && version.position == cell,
            Slot::CreatureReserved { planned, .. } => matches!(planned.as_ref(),
                Slot::CreatureOccupied { position: Some(version), .. }
                    if version.context == position_context && version.position == cell),
            Slot::NpcOccupied {
                position: version, ..
            } => version.context == position_context && version.position == cell,
            _ => false,
        })
    }

    /// True when `actor`, at its exact actor-local generation, is still this carrier's live
    /// creature (`health > 0`). A spawn cell may still name a removed creature (removal does
    /// not scan the spawn cells); the generation check keeps that stale reference from reading
    /// as the slot's later occupant.
    fn creature_alive(&self, continuity: &NamespaceContinuityGuard, actor: ExactActorRef) -> bool {
        self.validate_ref(continuity, actor.0).is_ok_and(|index| {
            matches!(
                self.slots[index],
                Slot::CreatureOccupied { generation, health: 1.., .. }
                    if generation == actor.0.actor_local_generation.0
            )
        })
    }

    /// The spawn source and cell index `actor` is (or, if now dead, was) admitted at, if any.
    /// The caller uses this right after a death commits, to schedule that cell's first respawn
    /// occurrence (attempt 1).
    fn locate_spawn_cell(&self, actor: ExactActorRef) -> Option<(SpawnSourceId, usize)> {
        self.spawns.iter().find_map(|spawn| {
            spawn
                .cells
                .iter()
                .position(|cell| cell.live == Some(actor))
                .map(|cell_index| (spawn.source, cell_index))
        })
    }

    /// This cell's current respawn-chain bookkeeping (attempt/successor and the actor, if any,
    /// it currently names) -- read-only, for the caller to build the next `RespawnOccurrence`.
    fn spawn_cell_state(
        &self,
        source: SpawnSourceId,
        cell_index: usize,
    ) -> Result<SpawnCellState, CarrierError> {
        let spawn = self
            .spawn_index(source)
            .map(|index| &self.spawns[index])
            .ok_or(CarrierError::UnknownSpawnSource)?;
        spawn
            .cells
            .get(cell_index)
            .copied()
            .ok_or(CarrierError::UnknownSpawnCell)
    }

    /// This spawn source's content-declared definition (respawn delay, occupancy retry
    /// interval), for the caller to compute the next occurrence's `due` deadline.
    fn spawn_definition(&self, source: SpawnSourceId) -> Result<&SpawnDefinition, CarrierError> {
        self.spawn_index(source)
            .map(|index| &self.spawns[index].definition)
            .ok_or(CarrierError::UnknownSpawnSource)
    }

    /// §4.3: applies one fired respawn timer for `(source, cell_index)`. If the cell is free,
    /// admits a fresh creature there (D52: a new actor-local generation; the dead actor's
    /// generation, if its slot still lingers, is retired first). If occupied, advances the
    /// cell's retry/successor bookkeeping and reports what the caller should schedule next
    /// (`RespawnResolution`); this method itself never calls `OwnerTimerLane::schedule` (that
    /// needs the caller's own fence-issued `RuntimeWorkStamp`, binding item (b)).
    /// The context is validated before any lookup or mutation. On the free-cell path, the
    /// dead actor's removal, the replacement's admission, its positioning and the cell's
    /// `live` update are staged and applied atomically: `self.slots`/`self.free_head` are
    /// snapshotted before the removal, and any failure from admission or positioning rolls
    /// the whole step back to that snapshot (so the cell keeps naming the original dead actor
    /// and no unpositioned replacement is left admitted) before returning the error.
    fn resolve_respawn_timer(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        source: SpawnSourceId,
        cell_index: usize,
        position_context: PreProductionPositionContext,
    ) -> Result<RespawnResolution, CarrierError> {
        self.validate_current_continuity(continuity)?;
        self.validate_position_context(position_context)?;
        let spawn_index = self
            .spawn_index(source)
            .ok_or(CarrierError::UnknownSpawnSource)?;
        let cell = *self.spawns[spawn_index]
            .definition
            .placement_cells
            .get(cell_index)
            .ok_or(CarrierError::UnknownSpawnCell)?;
        if self.cell_occupied(position_context, cell) {
            let state = &mut self.spawns[spawn_index].cells[cell_index];
            return Ok(if state.attempt >= AI01_SPAWN_OCCUPANCY_RETRIES_MAX {
                // Terminal disposition (§4.3): this chain ends `Skipped`; a fresh successor
                // chain starts at attempt 0/1 for the same cell, one full respawn delay later.
                state.attempt = 0;
                state.successor = state.successor.saturating_add(1);
                RespawnResolution::Skipped {
                    next_successor: state.successor,
                }
            } else {
                state.attempt = state.attempt.saturating_add(1);
                RespawnResolution::Postponed {
                    next_attempt: state.attempt,
                }
            });
        }

        let point = self.spawns[spawn_index].definition.point(cell_index)?;
        let target_identity = point.creature_target_identity.clone();
        let initial_health = point.creature_initial_health;
        let slots_before = self.slots.clone();
        let free_head_before = self.free_head;
        let occupied_before = self.occupied.clone();
        if let Some(dead_actor) = self.spawns[spawn_index].cells[cell_index].live {
            // Best effort: an already-removed actor (e.g. a repeated call) is not an error here.
            let _ = self.remove(continuity, dead_actor.0);
        }
        let actor = match self.admit_creature(
            continuity,
            ActorState(0),
            &target_identity,
            initial_health,
        ) {
            Ok(actor) => actor,
            Err(error) => {
                self.slots = slots_before;
                self.free_head = free_head_before;
                self.occupied = occupied_before;
                return Err(error);
            }
        };
        if let Err(error) = self.initialize_position(continuity, actor, position_context, cell) {
            self.slots = slots_before;
            self.free_head = free_head_before;
            self.occupied = occupied_before;
            return Err(error);
        }
        let state = &mut self.spawns[spawn_index].cells[cell_index];
        state.live = Some(ExactActorRef(actor));
        state.attempt = 0;
        Ok(RespawnResolution::Admitted(ExactActorRef(actor)))
    }

    /// SPAWN-CEILINGS-1 (Codex 4242270322): the respawn drive's admission at point
    /// `cell_index` of `source`, whose cell the caller has just found free under
    /// `position_context`. Unlike [`Self::resolve_respawn_timer`] it neither checks the cell
    /// again nor snapshots the slots: it costs the dead creature's removal (one pass over the
    /// occupied index) and one admission. Every refusal it can foresee (continuity, context,
    /// source, cell, the creature's health and identity, the identity's allocation) precedes the
    /// first mutation. The dead creature's removal is best effort, as in the timer resolution;
    /// after it only the creature ceiling `CREATUREAI0-RL-04` and the protected generation
    /// exhaustion of the freed slot (#541) can refuse, and that slot is retired either way. A
    /// refused position initialization removes the new creature again.
    fn respawn_into_free_cell(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        source: SpawnSourceId,
        cell_index: usize,
        position_context: PreProductionPositionContext,
    ) -> Result<ExactActorRef, CarrierError> {
        self.validate_current_continuity(continuity)?;
        self.validate_position_context(position_context)?;
        let spawn_index = self
            .spawn_index(source)
            .ok_or(CarrierError::UnknownSpawnSource)?;
        let realization = &self.spawns[spawn_index];
        let cell = *realization
            .definition
            .placement_cells
            .get(cell_index)
            .ok_or(CarrierError::UnknownSpawnCell)?;
        let dead_actor = realization
            .cells
            .get(cell_index)
            .ok_or(CarrierError::UnknownSpawnCell)?
            .live;
        let point = realization.definition.point(cell_index)?;
        let initial_health = point.creature_initial_health;
        if initial_health <= 0 {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        let target_identity = creature_target_binding(&point.creature_target_identity)?;
        if let Some(dead_actor) = dead_actor {
            // Best effort: an already-removed actor (e.g. a repeated call) is not an error here.
            let _ = self.remove(continuity, dead_actor.0);
        }
        // `CREATUREAI0-RL-04` (Codex 4242715014), as `admit_creature` checks it, counted after
        // the dead creature has left.
        if self.occupied.len() >= CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
        let actor = self.admit_inner(
            continuity,
            ActorState(0),
            None,
            true,
            Some((initial_health, target_identity)),
            false,
        )?;
        if let Err(error) = self.initialize_position(continuity, actor, position_context, cell) {
            let _ = self.remove(continuity, actor);
            return Err(error);
        }
        let state = &mut self.spawns[spawn_index].cells[cell_index];
        state.live = Some(ExactActorRef(actor));
        state.attempt = 0;
        Ok(ExactActorRef(actor))
    }
}

/// A creature's validated target identity, copied for its slot.
fn creature_target_binding(target_identity: &str) -> Result<Arc<[u8]>, CarrierError> {
    if target_identity.is_empty()
        || target_identity.len() > MAX_OWNER_COMMIT_BINDING_BYTES
        || !target_identity.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'.' | b'_' | b'-' | b'/')
        })
    {
        return Err(CarrierError::InvalidCreatureTarget);
    }
    Ok(Arc::from(copy_bounded_binding(target_identity.as_bytes())?))
}

fn copy_bounded_binding(bytes: &[u8]) -> Result<Box<[u8]>, CarrierError> {
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(bytes.len())
        .map_err(|_| CarrierError::CommitAllocationFailed)?;
    owned.extend_from_slice(bytes);
    Ok(owned.into_boxed_slice())
}

fn allocate_slots(explicit_capacity: usize) -> Result<Vec<Slot>, CarrierError> {
    #[cfg(any(test, feature = "test-support"))]
    if explicit_capacity == TEST_ALLOCATION_FAILURE_CAPACITY {
        return Err(CarrierError::AllocationFailed);
    }

    let mut slots = Vec::new();
    slots
        .try_reserve_exact(explicit_capacity)
        .map_err(|_| CarrierError::AllocationFailed)?;
    for index in 0..explicit_capacity {
        let next_free = if index + 1 < explicit_capacity {
            Some(u32::try_from(index + 1).map_err(|_| CarrierError::CapacityArithmeticOverflow)?)
        } else {
            None
        };
        slots.push(Slot::VacantReusable {
            generation: 0,
            next_free,
        });
    }
    Ok(slots)
}

/// MAP-ITEM-REF-1 Part B: the durable corpse Item a projected death materialized as. The Channel
/// owner binds it once the corpse's MINT has committed: KILL-REWARD part B calls
/// [`ChannelRuntimeV1::bind_corpse_item`] with the item instance id `commit_corpse_mint` returned
/// and the corpse's typed definition. Not durable, like the projection it belongs to: an unbound
/// corpse is neither shown nor usable, so nothing ever names a corpse Item by a made-up id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCorpseItem {
    pub item_instance_id: [u8; 16],
    pub family: String,
    pub production_key: String,
    pub revision_ref: String,
}

/// MAP-ITEM-REF-1 Part B: one bound corpse of [`ChannelRuntimeV1::visible_corpses`], at its
/// projected position. `identity` is [`ExactActorRef::corpse_identity`] of the dead actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleRuntimeCorpse {
    pub identity: [u8; 16],
    pub position: MovementLocalPosition,
    pub revision: u64,
    pub item: RuntimeCorpseItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CorpseItemBinding {
    actor: ExactActorRef,
    item: RuntimeCorpseItem,
}

impl ExactActorRef {
    /// MAP-ITEM-REF-1 Part B: the stable opaque identity of this actor's corpse, separate from
    /// [`Self::placement_identity`] so a corpse never shares an identity with the actor it was.
    pub fn corpse_identity(self) -> [u8; 16] {
        use sha2::{Digest, Sha256};
        let ActorRef {
            world_id,
            channel_id,
            scope_generation,
            actor_local_id,
            actor_local_generation,
        } = self.0;
        let digest = Sha256::new()
            .chain_update(b"oteryn:runtime-corpse-placement:v1")
            .chain_update(world_id.as_bytes())
            .chain_update(channel_id.as_bytes())
            .chain_update(scope_generation.get().to_be_bytes())
            .chain_update(actor_local_id.0.to_be_bytes())
            .chain_update(actor_local_generation.0.to_be_bytes())
            .finalize();
        let mut identity = [0_u8; 16];
        identity.copy_from_slice(&digest[..16]);
        identity
    }
}

impl ChannelActorCarrier {
    fn bind_corpse_item_inner(
        &mut self,
        actor: ExactActorRef,
        item: RuntimeCorpseItem,
    ) -> Result<(), CarrierError> {
        if actor.world_id() != self.world_id
            || actor.channel_id() != self.channel_id
            || actor.scope_generation() != self.scope_generation
            || !self
                .corpse_projections
                .iter()
                .any(|projection| projection.occurrence.actor == actor)
        {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        if let Some(bound) = self
            .corpse_item_bindings
            .iter()
            .find(|binding| binding.actor == actor)
        {
            return if bound.item == item {
                Ok(())
            } else {
                Err(CarrierError::CorpseProjectionConflict)
            };
        }
        if self
            .corpse_item_bindings
            .iter()
            .any(|binding| binding.item.item_instance_id == item.item_instance_id)
        {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        let projections = &self.corpse_projections;
        self.corpse_item_bindings.retain(|binding| {
            projections
                .iter()
                .any(|projection| projection.occurrence.actor == binding.actor)
        });
        self.corpse_item_bindings
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        self.corpse_item_bindings
            .push(CorpseItemBinding { actor, item });
        Ok(())
    }

    fn bound_corpses(
        &self,
        context: Option<PreProductionPositionContext>,
    ) -> Vec<VisibleRuntimeCorpse> {
        self.corpse_projections
            .iter()
            .filter(|projection| {
                context.is_none_or(|context| projection.position.context == context)
            })
            .filter_map(|projection| {
                let actor = projection.occurrence.actor;
                let binding = self
                    .corpse_item_bindings
                    .iter()
                    .find(|binding| binding.actor == actor)?;
                Some(VisibleRuntimeCorpse {
                    identity: actor.corpse_identity(),
                    position: projection.position(),
                    revision: projection.position_revision(),
                    item: binding.item.clone(),
                })
            })
            .collect()
    }
}

impl ChannelRuntimeV1 {
    /// MAP-ITEM-REF-1 Part B: bind `actor`'s projected corpse to the corpse Item its committed
    /// MINT created (`item_instance_id` and the typed definition `commit_corpse_mint` was given).
    /// The projection must be retained in this generation (`CommittedLethalUnavailable`
    /// otherwise); binding the same Item again is a no-op and another Item, or an Item bound to
    /// another corpse, is `CorpseProjectionConflict`. Bindings whose projection was evicted are
    /// dropped here, so there is at most one per retained projection.
    #[allow(
        dead_code,
        reason = "the interface KILL-REWARD part B calls after commit_corpse_mint"
    )]
    pub fn bind_corpse_item(
        &mut self,
        actor: ExactActorRef,
        item_instance_id: [u8; 16],
        family: &str,
        production_key: &str,
        revision_ref: &str,
    ) -> Result<(), CarrierError> {
        self.carrier.bind_corpse_item_inner(
            actor,
            RuntimeCorpseItem {
                item_instance_id,
                family: family.to_owned(),
                production_key: production_key.to_owned(),
                revision_ref: revision_ref.to_owned(),
            },
        )
    }

    /// Codex 4248192466 (LOOT-LIVE-1): keep `actor`'s retained corpse projection from eviction
    /// until [`Self::unpin_corpse_projection`], so the kill settlement the queue accepted for it
    /// can still bind its committed corpse Item. Pinning a pinned projection is a no-op. A
    /// projection that is not retained in this generation is `CommittedLethalUnavailable`; more
    /// than `MAX_PINNED_CORPSE_PROJECTIONS` pins are `CapacityExceeded`. Runtime memory only.
    pub fn pin_corpse_projection(&mut self, actor: ExactActorRef) -> Result<(), CarrierError> {
        self.carrier.pin_corpse_projection_inner(actor)
    }

    /// Codex 4248192466: release `actor`'s pin once its kill settlement is final. The projection
    /// stays retained until the bounded FIFO evicts it. Unpinning an unpinned actor is a no-op.
    pub fn unpin_corpse_projection(&mut self, actor: ExactActorRef) {
        self.carrier.unpin_corpse_projection_inner(actor);
    }

    /// Codex 4248192466: release every pin whose actor is not in `held`, the killed actors of the
    /// kill settlements still queued, parked or in flight. An entry freed without a settle (a
    /// `principal_gone` refusal) leaves its pin until this runs.
    pub fn retain_corpse_projection_pins(&mut self, held: &[ExactActorRef]) {
        self.carrier
            .corpse_projection_pins
            .retain(|pinned| held.contains(pinned));
    }

    /// Whether `actor`'s corpse projection is pinned against eviction.
    #[cfg(any(test, feature = "test-support"))]
    pub fn corpse_projection_pinned(&self, actor: ExactActorRef) -> bool {
        self.carrier.corpse_projection_pins.contains(&actor)
    }

    /// Whether `actor`'s committed death is still projected and retained in this generation.
    #[cfg(any(test, feature = "test-support"))]
    pub fn retains_corpse_projection(&self, actor: ExactActorRef) -> bool {
        self.carrier.retains_corpse_projection(actor)
    }

    /// MAP-ITEM-REF-1 Part B: every bound corpse of this generation under the pinned Movement
    /// context, in projection order. A corpse without a binding, or whose projection is gone, is
    /// left out. Read-only.
    pub fn visible_corpses(&self) -> Vec<VisibleRuntimeCorpse> {
        self.carrier
            .bound_corpses(Some(self.pinned_position_context()))
    }

    /// MAP-ITEM-REF-1 Part B: the bound corpse whose [`ExactActorRef::corpse_identity`] is
    /// `identity`, as [`Self::visible_corpses`] shows it. Read-only.
    pub fn bound_corpse(&self, identity: [u8; 16]) -> Option<VisibleRuntimeCorpse> {
        self.visible_corpses()
            .into_iter()
            .find(|corpse| corpse.identity == identity)
    }
}

#[cfg(any(test, feature = "test-support"))]
const TEST_ALLOCATION_FAILURE_CAPACITY: usize = u32::MAX as usize;

/// Foundation-only factory. Path-included Foundation integration crates do not import Content;
/// the Movement library tests supply real Content claims outside this module.
#[cfg(any(test, feature = "test-support"))]
pub struct MovementActorFixture {
    owner: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    actor: ExactActorRef,
}

#[cfg(any(test, feature = "test-support"))]
impl MovementActorFixture {
    pub fn new(position: MovementLocalPosition, creature: bool) -> Result<Self, CarrierError> {
        fn uuid(seed: u8) -> [u8; 16] {
            let mut bytes = [seed; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        }
        let grant = PreProductionContinuityGrant {
            world_id: WorldId::decode(&uuid(10)).map_err(|_| CarrierError::WrongScope)?,
            channel_id: ChannelId::decode(&uuid(11)).map_err(|_| CarrierError::WrongScope)?,
            scope_generation: ScopeOwnershipGeneration::new(1)
                .map_err(|_| CarrierError::WrongScope)?,
        };
        let mut owner = NamespaceContinuityGuard::from_pre_production_grant(grant);
        let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1)?;
        let actor = if creature {
            carrier.admit_creature(&owner, ActorState(1), "engineering:creature", 20)?
        } else {
            carrier.admit(&owner, ActorState(1))?
        };
        let context = PreProductionPositionContext {
            world_id: owner.world_id,
            channel_id: owner.channel_id,
            scope_generation: owner.current_generation,
            coordinate_frame_marker: 11,
            map_revision_marker: 12,
            content_generation_marker: 13,
        };
        carrier.initialize_position(
            &owner,
            actor,
            context,
            LocalPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
        )?;
        Ok(Self {
            owner,
            carrier,
            actor: ExactActorRef(actor),
        })
    }

    pub const fn world_id(&self) -> WorldId {
        self.owner.world_id
    }
    pub const fn actor(&self) -> ExactActorRef {
        self.actor
    }

    pub fn current_position(&self) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .read_movement_position(&self.owner, self.actor.0)
    }

    pub fn raw_position(&self) -> Result<(MovementLocalPosition, u64), CarrierError> {
        let version = self
            .carrier
            .read_position(&self.owner, self.actor.0)?
            .version;
        Ok((
            MovementLocalPosition {
                x: version.position.x,
                y: version.position.y,
                floor: version.position.floor,
            },
            version.revision,
        ))
    }

    pub fn borrow_position(&mut self) -> CurrentOwnerMovementPosition<'_> {
        self.carrier.current_owner_movement_position(&self.owner)
    }

    pub fn wrong_context(&self) -> Result<MovementPositionContext, CarrierError> {
        let mut context = self
            .carrier
            .read_position(&self.owner, self.actor.0)?
            .version
            .context;
        context.coordinate_frame_marker += 1;
        Ok(MovementPositionContext(context))
    }

    pub fn intervening_write(&mut self) -> Result<(), CarrierError> {
        let expected = self.carrier.read_position(&self.owner, self.actor.0)?;
        self.carrier.compare_commit_position(
            &self.owner,
            expected,
            expected.version.context,
            expected.version.position,
        )?;
        Ok(())
    }

    pub fn recycle(&mut self) -> Result<(), CarrierError> {
        let old = self.carrier.read_position(&self.owner, self.actor.0)?;
        self.carrier.remove(&self.owner, self.actor.0)?;
        let actor = self.carrier.admit(&self.owner, ActorState(2))?;
        self.carrier.initialize_position(
            &self.owner,
            actor,
            old.version.context,
            old.version.position,
        )?;
        self.actor = ExactActorRef(actor);
        Ok(())
    }

    pub fn become_creature(&mut self) -> Result<(), CarrierError> {
        let old = self.carrier.read_position(&self.owner, self.actor.0)?;
        self.carrier.remove(&self.owner, self.actor.0)?;
        let actor =
            self.carrier
                .admit_creature(&self.owner, ActorState(3), "engineering:creature", 20)?;
        self.carrier.initialize_position(
            &self.owner,
            actor,
            old.version.context,
            old.version.position,
        )?;
        self.actor = ExactActorRef(actor);
        Ok(())
    }

    pub fn advance_owner(&mut self) -> Result<(), CarrierError> {
        self.owner.advance(PreProductionContinuityGrant {
            world_id: self.owner.world_id,
            channel_id: self.owner.channel_id,
            scope_generation: ScopeOwnershipGeneration::new(2)
                .map_err(|_| CarrierError::WrongScope)?,
        })
    }

    pub fn exhaust_position_revision(&mut self) -> Result<(), CarrierError> {
        let index = self.carrier.validate_ref(&self.owner, self.actor.0)?;
        if let Slot::Occupied {
            position: Some(version),
            ..
        } = &mut self.carrier.slots[index]
        {
            version.revision = u64::MAX;
            Ok(())
        } else {
            Err(CarrierError::MovementCreatureUnavailable)
        }
    }
}

/// Test-only D1 death source (`VSL_COMBAT_FIXTURE_PROFILE`): one positioned
/// fixture creature in a pre-production Channel carrier of the given scope
/// generation. A death exists only through the owner's committed lethal path
/// and Combat's projection; the fixture returns the death key that projection
/// names and never constructs one. Nothing here exists outside `cfg(test)`.
#[cfg(any(test, feature = "test-support"))]
pub struct CombatDeathFixture {
    owner: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    actor: ExactActorRef,
    /// D4: `strike_by` derives one fixture command per distinct occurrence text, so the same
    /// text replays as the same `(session, sequence)` command (test-only bookkeeping).
    strike_commands: Vec<(String, u64)>,
    /// ATTACK-1b: the bound player attacker of [`Self::swing`], when built with one.
    attacker: Option<ExactActorRef>,
}

#[cfg(any(test, feature = "test-support"))]
impl CombatDeathFixture {
    /// Fixture creature health: test/evidence only, not Reference behavior.
    pub const HEALTH: i64 = 20;
    const TARGET: &'static str = "fixture:vsl-combat.creature";
    pub const POSITION: LocalPosition = LocalPosition {
        x: 100,
        y: 100,
        floor: 7,
    };

    pub fn new(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
    ) -> Result<Self, CarrierError> {
        Self::new_with_health(world_id, channel_id, scope_generation, Self::HEALTH)
    }

    /// Test-only source-health variant; default fixture and all production paths stay intact.
    pub fn new_with_health(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
        health: i64,
    ) -> Result<Self, CarrierError> {
        Self::build(world_id, channel_id, scope_generation, health, 1)
    }

    /// ATTACK-1b: the fixture creature plus one bound player attacker (Character `character`,
    /// lease generation 1) of the fixture swing session, for [`Self::swing`].
    pub fn new_with_bound_attacker(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
        character: CharacterId,
    ) -> Result<Self, CarrierError> {
        let mut fixture = Self::build(world_id, channel_id, scope_generation, Self::HEALTH, 2)?;
        let lease =
            CharacterLease::new(character, 1).map_err(|_| CarrierError::InvalidActorIdentity)?;
        let attacker = fixture.carrier.admit_bound_test_attacker(
            &fixture.owner,
            Self::swing_session()?,
            lease,
        )?;
        fixture.attacker = Some(attacker);
        Ok(fixture)
    }

    fn build(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
        health: i64,
        capacity: usize,
    ) -> Result<Self, CarrierError> {
        let mut owner =
            NamespaceContinuityGuard::from_pre_production_grant(PreProductionContinuityGrant {
                world_id,
                channel_id,
                scope_generation,
            });
        let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut owner, capacity)?;
        let actor = carrier.admit_creature(&owner, ActorState(1), Self::TARGET, health)?;
        let context = PreProductionPositionContext {
            world_id,
            channel_id,
            scope_generation,
            coordinate_frame_marker: 41,
            map_revision_marker: 42,
            content_generation_marker: 43,
        };
        carrier.initialize_position(&owner, actor, context, Self::POSITION)?;
        Ok(Self {
            owner,
            carrier,
            actor: ExactActorRef(actor),
            strike_commands: Vec::new(),
            attacker: None,
        })
    }

    fn swing_session() -> Result<GameSessionId, CarrierError> {
        let mut session = [0_u8; 16];
        session[6] = 0x70;
        session[8] = 0x80;
        session[15] = 2;
        GameSessionId::decode(&session).map_err(|_| CarrierError::InvalidActorIdentity)
    }

    /// ATTACK-1b: one auto-attack swing of the bound attacker, committed under the swing
    /// identity `(lineage command `lineage`, swing_ordinal)` exactly as the Channel owner's
    /// auto-attack drain commits it.
    pub fn swing(
        &mut self,
        lineage: u64,
        swing_ordinal: u16,
        damage: i64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let attacker = self.attacker.ok_or(CarrierError::InvalidActorIdentity)?;
        let lineage = CommandRef::new(
            Self::swing_session()?,
            super::CommandId::new(lineage).map_err(|_| CarrierError::InvalidActorIdentity)?,
        );
        let binding = format!(
            "fixture:attack.swing.v1\0{}\0{swing_ordinal}",
            lineage.command_id().get()
        );
        self.carrier
            .current_owner_exact_commit(&self.owner)
            .commit_swing_damage_for_bound_attacker(
                self.actor,
                attacker,
                lineage,
                swing_ordinal,
                OwnerDamageCommand {
                    target: Self::TARGET.as_bytes(),
                    occurrence: &[],
                    binding: binding.as_bytes(),
                    damage,
                },
            )
    }

    pub const fn actor(&self) -> ExactActorRef {
        self.actor
    }

    /// One owner-committed damage occurrence. An identical replay returns the
    /// recorded transition without a second write.
    pub fn strike(
        &mut self,
        occurrence: &str,
        damage: i64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let mut binding = occurrence.as_bytes().to_vec();
        binding.extend_from_slice(b"\0fixture:vsl-combat.strike.v1");
        self.carrier
            .current_owner_exact_commit(&self.owner)
            .commit_damage(
                self.actor,
                OwnerDamageCommand {
                    target: Self::TARGET.as_bytes(),
                    occurrence: occurrence.as_bytes(),
                    binding: &binding,
                    damage,
                },
            )
    }

    /// MAP-ITEM-REF-1 Part B: [`ChannelRuntimeV1::bind_corpse_item`] for the fixture's corpse.
    pub fn bind_corpse_item(
        &mut self,
        item_instance_id: [u8; 16],
        family: &str,
        production_key: &str,
        revision_ref: &str,
    ) -> Result<(), CarrierError> {
        self.carrier.bind_corpse_item_inner(
            self.actor,
            RuntimeCorpseItem {
                item_instance_id,
                family: family.to_owned(),
                production_key: production_key.to_owned(),
                revision_ref: revision_ref.to_owned(),
            },
        )
    }

    /// MAP-ITEM-REF-1 Part B: the fixture's bound corpse Item, if bound.
    pub fn bound_corpse_item_instance_id(&self) -> Option<[u8; 16]> {
        self.carrier
            .bound_corpses(None)
            .into_iter()
            .find(|corpse| corpse.identity == self.actor.corpse_identity())
            .map(|corpse| corpse.item.item_instance_id)
    }

    /// D132/D3-3 + D4: the same owner-committed damage occurrence as [`Self::strike`],
    /// additionally attributed to `attacker`'s running per-creature damage total. The fixture
    /// stands in for the future transport: it maps each distinct `occurrence` text to one
    /// monotonic `CommandId` of one fixed fixture `GameSessionId`, so the same text replays as
    /// the same command and a new text is a new, higher-sequence command.
    pub fn strike_by(
        &mut self,
        occurrence: &str,
        damage: i64,
        attacker: CharacterId,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let sequence = match self
            .strike_commands
            .iter()
            .find(|(text, _)| text == occurrence)
        {
            Some((_, sequence)) => *sequence,
            None => {
                let sequence = u64::try_from(self.strike_commands.len())
                    .map_err(|_| CarrierError::CapacityArithmeticOverflow)?
                    + 1;
                self.strike_commands.push((occurrence.to_owned(), sequence));
                sequence
            }
        };
        let mut session = [0_u8; 16];
        session[6] = 0x70;
        session[8] = 0x80;
        session[15] = 1;
        let command = CommandRef::new(
            GameSessionId::decode(&session).map_err(|_| CarrierError::InvalidActorIdentity)?,
            super::CommandId::new(sequence).map_err(|_| CarrierError::InvalidActorIdentity)?,
        );
        let mut binding = occurrence.as_bytes().to_vec();
        binding.extend_from_slice(b"\0fixture:vsl-combat.strike.v1");
        self.carrier
            .current_owner_exact_commit(&self.owner)
            .commit_damage_for_attacker(
                self.actor,
                AttackerCommand::new(attacker, 1, command, 0),
                OwnerDamageCommand {
                    target: Self::TARGET.as_bytes(),
                    occurrence: occurrence.as_bytes(),
                    binding: &binding,
                    damage,
                },
            )
    }

    /// D132/D3-3: the current deterministic top-damage `CharacterId` tracked for this fixture's
    /// creature, [`CurrentOwnerCombatDeath::top_damage_character`].
    pub fn top_damage_character(&mut self) -> Result<Option<CharacterId>, CarrierError> {
        self.carrier
            .current_owner_combat_death(&self.owner)
            .top_damage_character(self.actor)
    }

    /// Administrative despawn: the creature leaves without a semantic death.
    pub fn despawn(&mut self) -> Result<(), CarrierError> {
        self.carrier.remove(&self.owner, self.actor.0).map(|_| ())
    }

    /// D2b: the same current-owner Combat-death capability
    /// [`ChannelRuntimeV1::borrow_combat_death`] grants, for
    /// `reward_occurrence` tests.
    pub fn borrow_combat_death(&mut self) -> CurrentOwnerCombatDeath<'_> {
        self.carrier.current_owner_combat_death(&self.owner)
    }

    /// The scope moved: owner continuity advances to `next`, while this
    /// carrier still serves its original generation.
    pub fn advance_owner(&mut self, next: ScopeOwnershipGeneration) -> Result<(), CarrierError> {
        self.owner.advance(PreProductionContinuityGrant {
            world_id: self.owner.world_id,
            channel_id: self.owner.channel_id,
            scope_generation: next,
        })
    }
}

#[cfg(any(test, feature = "test-support"))]
impl ChannelActorCarrier {
    /// Test only: a committed player slot for `session` with `lease` bound, as fresh admission
    /// leaves it (A2).
    pub fn admit_bound_test_attacker(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        session: GameSessionId,
        lease: CharacterLease,
    ) -> Result<ExactActorRef, CarrierError> {
        let reservation = self.reserve_player(continuity, session)?;
        let actor = self.commit_reserved_player(continuity, reservation)?;
        self.bind_attacker_lease(continuity, actor.0, session, lease)?;
        Ok(actor)
    }

    /// Test only: the committed player slot of `session`.
    pub fn test_player_actor(&self, session: GameSessionId) -> Option<ExactActorRef> {
        self.slots
            .iter()
            .enumerate()
            .find_map(|(index, slot)| match slot {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(bound),
                    committed: true,
                    ..
                } if *bound == session => Some(ExactActorRef(ActorRef {
                    world_id: self.world_id,
                    channel_id: self.channel_id,
                    scope_generation: self.scope_generation,
                    actor_local_id: ActorLocalId(u32::try_from(index + 1).ok()?),
                    actor_local_generation: ActorLocalGeneration(*generation),
                })),
                _ => None,
            })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "damage_contributors_tests.rs"]
mod damage_contributors_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "channel_actor_position_tests.rs"]
mod channel_actor_position_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut value = [0; 16];
        value[8..].copy_from_slice(&raw.to_be_bytes());
        value[6] = 0x70;
        value[8] = (value[8] & 0x3f) | 0x80;
        value
    }

    fn grant(seed: u64, generation: u64) -> PreProductionContinuityGrant {
        PreProductionContinuityGrant {
            world_id: WorldId::decode(&uuid_v7(seed)).expect("valid WorldId fixture"),
            channel_id: ChannelId::decode(&uuid_v7(seed + 1)).expect("valid ChannelId fixture"),
            scope_generation: ScopeOwnershipGeneration::new(generation)
                .expect("non-zero generation fixture"),
        }
    }

    fn continuity(seed: u64, generation: u64) -> NamespaceContinuityGuard {
        NamespaceContinuityGuard::from_pre_production_grant(grant(seed, generation))
    }

    fn carrier(capacity: usize) -> (NamespaceContinuityGuard, ChannelActorCarrier) {
        let mut continuity = continuity(10, 1);
        let carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, capacity)
            .expect("valid explicit test capacity");
        (continuity, carrier)
    }

    fn prove_boundary(capacity: usize) {
        let (continuity, mut carrier) = carrier(capacity);
        let mut refs = Vec::new();
        for value in 0..capacity {
            refs.push(
                carrier
                    .admit(&continuity, ActorState(value as u64))
                    .expect("M succeeds"),
            );
        }
        let before = carrier.slots.clone();
        assert_eq!(
            carrier.admit(&continuity, ActorState(99)),
            Err(CarrierError::CapacityExceeded)
        );
        assert_eq!(carrier.slots, before);
        for actor_ref in refs {
            assert!(carrier.lookup(&continuity, actor_ref).is_ok());
        }
    }

    pub(super) fn session(raw: u64) -> GameSessionId {
        GameSessionId::decode(&uuid_v7(raw)).expect("valid GameSessionId fixture")
    }

    fn node(raw: u64) -> NodeId {
        NodeId::decode(&uuid_v7(raw)).expect("valid NodeId fixture")
    }

    pub(super) fn runtime(capacity: usize) -> ChannelRuntimeV1 {
        ChannelRuntimeV1::from_committed_assignment(
            WorldId::decode(&uuid_v7(20)).expect("world"),
            ChannelId::decode(&uuid_v7(21)).expect("channel"),
            node(22),
            7,
            3,
            11,
            "runtime-scope-assignment:11",
            capacity,
            ChannelContentPin::test(WorldId::decode(&uuid_v7(20)).expect("world")),
        )
        .expect("runtime")
    }

    #[test]
    fn composed_runtime_binds_assignment_and_reserves_before_commit() {
        let mut runtime = runtime(1);
        let binding = runtime.binding();
        assert!(binding.matches_committed_assignment(
            binding.world_id(),
            binding.channel_id(),
            node(22),
            7,
            3,
            11,
            "runtime-scope-assignment:11",
        ));
        assert!(!binding.matches_committed_assignment(
            binding.world_id(),
            binding.channel_id(),
            node(22),
            7,
            4,
            12,
            "runtime-scope-assignment:12",
        ));

        let first_session = session(30);
        let reservation = runtime
            .reserve_fresh_session(first_session)
            .expect("M reservation succeeds");
        assert_eq!(
            runtime.reserve_fresh_session(session(31)),
            Err(CarrierError::CapacityExceeded)
        );
        let actor = runtime
            .commit_fresh_session(reservation)
            .expect("durable success commits actor");
        assert!(runtime.contains_committed_session(first_session, actor));
        assert_eq!(
            runtime.remove_terminal_session(session(31), actor),
            Err(CarrierError::PlayerReservationMismatch)
        );
        runtime
            .remove_terminal_session(first_session, actor)
            .expect("authoritative terminal cleanup");
        assert!(!runtime.contains_committed_session(first_session, actor));
    }

    #[test]
    fn qualification_spawn_uses_current_policy_and_refuses_wrong_pin_occupancy_duplicates_retirement()
     {
        use runtime_actor_companion::{
            CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
        };
        let mut current = runtime(4);
        let pin = current.content_pin().clone();
        let position = MovementLocalPosition {
            x: 2,
            y: 0,
            floor: 0,
        };
        assert!(
            current
                .realize_native_qualification_spawn(pin.clone(), "fixture:rat", position)
                .is_err()
        );
        let policy = CompiledCreaturePolicy {
            definition_key: "fixture:rat".into(),
            definition_revision: "fixture:1".into(),
            display_name: "fixture Rat".into(),
            maximum_health: 37,
            base_speed: 110,
            outfit_look_type: 21,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: vec![],
            defense: Some(1),
            armor: Some(1),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![],
            preferred_distance: Some(1),
            reward_boss: Some(false),
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        };
        current
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    pin.server_artifact_digest(),
                    vec![policy],
                )
                .expect("qualified fixture table"),
            )
            .expect("current policy owner");
        let reserved = current
            .reserve_fresh_session(session(900))
            .expect("ordinary session reservation");
        let player = current
            .commit_fresh_session(reserved)
            .expect("actual player actor");
        current
            .initialize_first_entry_position(player)
            .expect("actual positioned owner");
        let before = current.positioned_actor_census().expect("current census");
        let mut other_pin = pin.clone();
        other_pin.activation_sequence += 1;
        assert!(
            current
                .realize_native_qualification_spawn(other_pin, "fixture:rat", position)
                .is_err()
        );
        assert!(
            current
                .realize_native_qualification_spawn(
                    pin.clone(),
                    "fixture:rat",
                    current
                        .read_actor_position(player)
                        .expect("actual player position")
                        .position()
                )
                .is_err()
        );
        assert_eq!(
            current.positioned_actor_census().expect("unchanged census"),
            before
        );
        let actual = current
            .realize_native_qualification_spawn(pin.clone(), "fixture:rat", position)
            .expect("actual spawn primitive");
        assert_eq!(
            (actual.health, actual.maximum_health),
            (37, 37),
            "HP comes from current policy rather than20"
        );
        assert!(actual.state.master.is_none());
        assert_eq!(actual.position, position);
        let spawned = current
            .positioned_actor_census()
            .expect("actual creature census");
        assert_eq!(
            current.realize_native_qualification_spawn(
                pin.clone(),
                "fixture:rat",
                MovementLocalPosition { x: 3, ..position }
            ),
            Err(CarrierError::DuplicateSpawnSource)
        );
        assert_eq!(
            current
                .positioned_actor_census()
                .expect("no partial duplicate admission"),
            spawned
        );
        current.retire_owner_cycle();
        assert!(
            current
                .realize_native_qualification_spawn(pin.clone(), "fixture:rat", position)
                .is_err()
        );
    }

    #[test]
    fn definitely_uncommitted_reservation_rolls_back_but_committed_does_not() {
        let mut runtime = runtime(1);
        let first_session = session(40);
        let reservation = runtime
            .reserve_fresh_session(first_session)
            .expect("reserve");
        runtime
            .rollback_definitely_uncommitted(reservation)
            .expect("definite noncommit rollback");
        let second = runtime
            .reserve_fresh_session(session(41))
            .expect("capacity restored");
        let actor = runtime.commit_fresh_session(second).expect("commit");
        assert_eq!(
            runtime.rollback_definitely_uncommitted(second),
            Err(CarrierError::PlayerReservationMismatch)
        );
        assert!(runtime.contains_committed_session(session(41), actor));
    }

    #[test]
    fn control_loss_is_recorded_once_for_the_exact_committed_player() {
        let mut runtime = runtime(2);
        let reservation = runtime.reserve_fresh_session(session(64)).expect("reserve");
        // An uncommitted reservation has no player facts and cannot lose control.
        let _pending = runtime.reserve_fresh_session(session(65)).expect("reserve");
        let actor = runtime.commit_fresh_session(reservation).expect("commit");
        let mark = ControlLossMark {
            epoch: 1,
            grace_deadline: 160,
        };
        let facts = runtime
            .player_control_facts(actor, session(64))
            .expect("facts");
        assert_eq!(facts.control_loss, None);
        assert_ne!(facts.placement_identity, [0; 16]);
        assert_eq!(facts.placement_revision, actor.0.actor_local_generation.0);
        // Another session never reads or marks this actor.
        assert_eq!(
            runtime.player_control_facts(actor, session(66)),
            Err(CarrierError::PlayerReservationMismatch)
        );
        assert_eq!(
            runtime.record_control_loss(actor, session(66), mark),
            Err(CarrierError::PlayerReservationMismatch)
        );
        assert_eq!(
            runtime.record_control_loss(actor, session(64), ControlLossMark { epoch: 0, ..mark }),
            Err(CarrierError::ControlLossConflict)
        );
        assert_eq!(
            runtime.record_control_loss(actor, session(64), mark),
            Ok(())
        );
        // The identical decision replays; a different one conflicts and changes nothing.
        assert_eq!(
            runtime.record_control_loss(actor, session(64), mark),
            Ok(())
        );
        assert_eq!(
            runtime.record_control_loss(
                actor,
                session(64),
                ControlLossMark {
                    grace_deadline: 161,
                    ..mark
                }
            ),
            Err(CarrierError::ControlLossConflict)
        );
        let after = runtime
            .player_control_facts(actor, session(64))
            .expect("facts");
        assert_eq!(after.control_loss, Some(mark));
        assert_eq!(after.placement_identity, facts.placement_identity);
        assert_eq!(runtime.player_control_loss_epochs(), vec![1]);
        // The actor stays present: the committed player count is unchanged.
        assert_eq!(runtime.player_slot_counts(), (1, 1));
        // A committed same-session recovery restores control of the same actor; only
        // the exact recorded epoch clears it, and restoring again is idempotent.
        assert_eq!(
            runtime.restore_control(actor, session(64), 2),
            Err(CarrierError::ControlLossConflict)
        );
        assert_eq!(
            runtime.restore_control(actor, session(65), 1),
            Err(CarrierError::PlayerReservationMismatch)
        );
        assert_eq!(runtime.restore_control(actor, session(64), 1), Ok(()));
        assert_eq!(runtime.player_control_loss_epochs(), Vec::<u64>::new());
        assert_eq!(runtime.restore_control(actor, session(64), 1), Ok(()));
        assert_eq!(runtime.player_slot_counts(), (1, 1));
    }

    #[test]
    fn first_entry_position_is_written_once_and_retry_reconciles() {
        let mut runtime = runtime(2);
        let reservation = runtime.reserve_fresh_session(session(60)).expect("reserve");
        let actor = runtime.commit_fresh_session(reservation).expect("commit");
        let outcome = runtime
            .initialize_first_entry_position(actor)
            .expect("initialize");
        assert!(matches!(outcome, FirstEntryPosition::Initialized(_)));
        let first = runtime
            .borrow_movement_position()
            .read(actor)
            .expect("positioned");
        assert_eq!(
            first.position(),
            MovementLocalPosition {
                x: 0,
                y: 0,
                floor: 0
            }
        );
        assert_eq!(first.revision(), 1);
        assert_eq!(runtime.players_positioned_at_entry_start(), 1);
        // Exact retry: same actor generation, same pin -> no second write.
        assert_eq!(
            runtime.initialize_first_entry_position(actor),
            Ok(FirstEntryPosition::Reconciled(first))
        );
        assert_eq!(runtime.players_positioned_at_entry_start(), 1);
    }

    #[test]
    fn first_entry_position_refuses_other_context_and_stale_actor() {
        let mut runtime = runtime(2);
        // An actor positioned under another context never gets a replacement.
        let reservation = runtime.reserve_fresh_session(session(61)).expect("reserve");
        let other = runtime.commit_fresh_session(reservation).expect("commit");
        let synthetic = runtime
            .initialize_movement_test_position(
                other,
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
            )
            .expect("synthetic position");
        assert_eq!(
            runtime.initialize_first_entry_position(other),
            Err(CarrierError::PositionAlreadyInitialized)
        );
        assert_eq!(
            runtime.borrow_movement_position().read(other),
            Ok(synthetic)
        );
        // A removed (stale) actor reference gets no write.
        let reservation = runtime.reserve_fresh_session(session(62)).expect("reserve");
        let stale = runtime.commit_fresh_session(reservation).expect("commit");
        runtime
            .remove_terminal_session(session(62), stale)
            .expect("terminal cleanup");
        assert!(runtime.initialize_first_entry_position(stale).is_err());
        assert_eq!(runtime.players_positioned_at_entry_start(), 0);
    }

    #[test]
    fn first_entry_position_refuses_wrong_scope_and_recycled_reference() {
        // A reference from another Channel runtime is refused without a write.
        let mut runtime = runtime(1);
        let mut other = ChannelRuntimeV1::from_committed_assignment(
            WorldId::decode(&uuid_v7(70)).expect("world"),
            ChannelId::decode(&uuid_v7(71)).expect("channel"),
            node(72),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            1,
            ChannelContentPin::test(WorldId::decode(&uuid_v7(70)).expect("world")),
        )
        .expect("other runtime");
        let reservation = other.reserve_fresh_session(session(73)).expect("reserve");
        let foreign = other.commit_fresh_session(reservation).expect("commit");
        assert!(runtime.initialize_first_entry_position(foreign).is_err());
        assert_eq!(runtime.players_positioned_at_entry_start(), 0);

        // A recycled slot: the old reference is refused; the new actor starts
        // unpositioned and gets its own first write.
        let reservation = runtime.reserve_fresh_session(session(74)).expect("reserve");
        let old = runtime.commit_fresh_session(reservation).expect("commit");
        runtime
            .remove_terminal_session(session(74), old)
            .expect("terminal cleanup");
        let reservation = runtime.reserve_fresh_session(session(75)).expect("reserve");
        let recycled = runtime.commit_fresh_session(reservation).expect("commit");
        assert!(runtime.initialize_first_entry_position(old).is_err());
        assert!(matches!(
            runtime.initialize_first_entry_position(recycled),
            Ok(FirstEntryPosition::Initialized(_))
        ));
        assert_eq!(runtime.players_positioned_at_entry_start(), 1);
    }

    #[test]
    fn runtime_rejects_invalid_assignment_identity_and_zero_provenance() {
        let world = WorldId::decode(&uuid_v7(50)).expect("world");
        let channel = ChannelId::decode(&uuid_v7(51)).expect("channel");
        assert!(matches!(
            ChannelRuntimeV1::from_committed_assignment(
                world,
                channel,
                node(52),
                0,
                1,
                1,
                "runtime-scope-assignment:1",
                1,
                ChannelContentPin::test(world),
            ),
            Err(CarrierError::InvalidAssignmentBinding)
        ));
        assert!(matches!(
            ChannelRuntimeV1::from_committed_assignment(
                world,
                channel,
                node(52),
                1,
                1,
                2,
                "runtime-scope-assignment:1",
                1,
                ChannelContentPin::test(world),
            ),
            Err(CarrierError::InvalidAssignmentBinding)
        ));
        let other_world = WorldId::decode(&uuid_v7(53)).expect("other world");
        assert!(matches!(
            ChannelRuntimeV1::from_committed_assignment(
                world,
                channel,
                node(52),
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                1,
                ChannelContentPin::test(other_world),
            ),
            Err(CarrierError::ContentPinWorldMismatch)
        ));
        let runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node(52),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            1,
            ChannelContentPin::test(world),
        )
        .expect("pinned runtime");
        assert_eq!(runtime.content_pin(), &ChannelContentPin::test(world));
        assert_eq!(
            size_of::<Slot>(),
            // D132/D3-3: +8 bytes (one pointer) for CreatureOccupied's boxed
            // `damage_contributors`, the same footprint-preserving pattern already used for
            // `target_identity: Arc<[u8]>` above; every non-creature slot pays this one pointer
            // too, since it is `Slot`'s largest-variant size, not per-variant.
            // D4/D140: `committed` widens from the inline `Option<OwnerCommitRecord>` to a boxed
            // `DamageReceipts` (up to 16 receipts + the owner ordinal live on the heap), which
            // shrinks the largest variant by 32 bytes: 200 -> 168.
            // COND-1c: boxing the player lifecycle replaces the inline control-loss mark with
            // one pointer and reduces the largest variant by another 8 bytes: 168 -> 160.
            // D314: SPELL-BATCH candidate budget of 184 bytes for the boxed actual spell combat
            // state and qualified companion metadata on top of COND-1c; measured 160 -> 176.
            176,
            "spell candidate fixed-slot footprint must stay inside the D314 budget"
        );
    }

    #[test]
    fn multiple_explicit_fixture_bounds_enforce_m_and_m_plus_one() {
        prove_boundary(1);
        prove_boundary(2);
        prove_boundary(4);
    }

    #[test]
    fn construction_rejects_zero_and_checked_overflow() {
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity(10, 1), 0),
            Err(CarrierError::InvalidCapacity)
        );
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity(10, 1), usize::MAX),
            Err(CarrierError::CapacityArithmeticOverflow)
        );
    }

    #[test]
    fn exact_lookup_rejects_cross_scope_and_invalid_identity() {
        let (continuity, mut carrier) = carrier(1);
        let actor_ref = carrier.admit(&continuity, ActorState(7)).expect("admit");
        assert_eq!(carrier.lookup(&continuity, actor_ref), Ok(&ActorState(7)));

        let mut wrong_world = actor_ref;
        wrong_world.world_id = grant(20, 1).world_id;
        assert_eq!(
            carrier.lookup(&continuity, wrong_world),
            Err(CarrierError::WrongScope)
        );
        let mut wrong_channel = actor_ref;
        wrong_channel.channel_id = grant(20, 1).channel_id;
        assert_eq!(
            carrier.lookup(&continuity, wrong_channel),
            Err(CarrierError::WrongScope)
        );
        let mut wrong_generation = actor_ref;
        wrong_generation.scope_generation = ScopeOwnershipGeneration::new(2).expect("valid");
        assert_eq!(
            carrier.lookup(&continuity, wrong_generation),
            Err(CarrierError::WrongScope)
        );
        let mut missing = actor_ref;
        missing.actor_local_id = ActorLocalId(2);
        assert_eq!(
            carrier.lookup(&continuity, missing),
            Err(CarrierError::InvalidActorIdentity)
        );
    }

    #[test]
    fn removal_retains_generation_and_reuse_stales_old_reference() {
        let (continuity, mut carrier) = carrier(1);
        let first = carrier
            .admit(&continuity, ActorState(1))
            .expect("first admit");
        assert_eq!(carrier.remove(&continuity, first), Ok(ActorState(1)));
        assert_eq!(
            carrier.lookup(&continuity, first),
            Err(CarrierError::StaleActorGeneration)
        );
        let second = carrier.admit(&continuity, ActorState(2)).expect("reuse");
        assert_eq!(second.actor_local_id, first.actor_local_id);
        assert_eq!(second.actor_local_generation.0, 2);
        assert_eq!(
            carrier.lookup(&continuity, first),
            Err(CarrierError::StaleActorGeneration)
        );
    }

    #[test]
    fn multiple_removed_holes_reuse_in_lifo_removal_order() {
        let (continuity, mut carrier) = carrier(4);
        let refs = (0..4)
            .map(|value| {
                carrier
                    .admit(&continuity, ActorState(value))
                    .expect("initial admit")
            })
            .collect::<Vec<_>>();

        assert_eq!(carrier.remove(&continuity, refs[1]), Ok(ActorState(1)));
        assert_eq!(carrier.remove(&continuity, refs[3]), Ok(ActorState(3)));

        let first_reuse = carrier
            .admit(&continuity, ActorState(10))
            .expect("first recycled admit");
        let second_reuse = carrier
            .admit(&continuity, ActorState(11))
            .expect("second recycled admit");

        assert_eq!(first_reuse.actor_local_id, refs[3].actor_local_id);
        assert_eq!(second_reuse.actor_local_id, refs[1].actor_local_id);
        assert_eq!(first_reuse.actor_local_generation.0, 2);
        assert_eq!(second_reuse.actor_local_generation.0, 2);
    }

    #[test]
    fn representable_post_selection_failure_rolls_back_everything() {
        let (continuity, mut carrier) = carrier(2);
        let first = carrier
            .admit(&continuity, ActorState(1))
            .expect("unrelated actor");
        let before = carrier.slots.clone();
        let free_head_before = carrier.free_head;
        assert_eq!(
            carrier.admit_inner(&continuity, ActorState(2), None, true, None, true),
            Err(CarrierError::InjectedAdmissionFailure)
        );
        assert_eq!(carrier.slots, before);
        assert_eq!(carrier.free_head, free_head_before);
        assert_eq!(carrier.lookup(&continuity, first), Ok(&ActorState(1)));
    }

    #[test]
    fn exhausted_reuse_marks_only_selected_slot_and_never_reselects_it() {
        let (continuity, mut carrier) = carrier(2);
        assert!(matches!(carrier.slots[0], Slot::VacantReusable { .. }));
        if let Slot::VacantReusable { generation, .. } = &mut carrier.slots[0] {
            *generation = u64::MAX;
        }
        let unrelated = carrier.slots[1].clone();
        assert_eq!(
            carrier.admit(&continuity, ActorState(1)),
            Err(CarrierError::ActorGenerationExhausted)
        );
        assert_eq!(
            carrier.slots[0],
            Slot::Exhausted {
                generation: u64::MAX
            }
        );
        assert_eq!(carrier.slots[1], unrelated);
        let admitted = carrier
            .admit(&continuity, ActorState(2))
            .expect("skip exhausted slot");
        assert_eq!(admitted.actor_local_id, ActorLocalId(2));
    }

    #[test]
    fn ai2_multiple_creatures_coexist_bounded_only_by_general_capacity() {
        // AI-2 (GAME-AI-01 §4.1: the carrier's one-creature limit is lifted to the D57
        // envelope): a second, distinct creature now admits successfully while the first is
        // still live, bounded only by the carrier's own general slot capacity.
        let (continuity, mut carrier) = carrier(2);

        let first = carrier
            .admit_creature(&continuity, ActorState(1), "target:one", 20)
            .expect("first creature");
        let second = carrier
            .admit_creature(&continuity, ActorState(2), "target:two", 20)
            .expect("second creature coexists with the first");
        assert_ne!(first, second);
        assert_eq!(
            carrier.admit_creature(&continuity, ActorState(3), "target:three", 20),
            Err(CarrierError::CapacityExceeded),
            "capacity 2 is exhausted by two live creatures"
        );

        assert_eq!(carrier.remove(&continuity, first), Ok(ActorState(1)));
        carrier
            .admit_creature(&continuity, ActorState(4), "target:four", 20)
            .expect("the freed slot is reusable while the other creature stays live");
        assert!(
            carrier
                .current_owner_exact_lookup(&continuity)
                .contains(ExactActorRef(second))
        );
    }

    fn spawn_position_context(
        continuity: &NamespaceContinuityGuard,
    ) -> PreProductionPositionContext {
        PreProductionPositionContext {
            world_id: continuity.world_id,
            channel_id: continuity.channel_id,
            scope_generation: continuity.current_generation,
            coordinate_frame_marker: 1,
            map_revision_marker: 1,
            content_generation_marker: 1,
        }
    }

    #[test]
    fn evicting_a_corpse_projection_evicts_its_reward_occurrence() {
        let (continuity, mut carrier) = carrier(4);
        let context = spawn_position_context(&continuity);
        let actor = |local: u32| ActorRef {
            world_id: continuity.world_id,
            channel_id: continuity.channel_id,
            scope_generation: continuity.current_generation,
            actor_local_id: ActorLocalId(local),
            actor_local_generation: ActorLocalGeneration(1),
        };
        let projection = |local: u32| RuntimeCorpseProjection {
            death_at_ms: None,
            occurrence: CreatureDeathOccurrenceRef {
                actor: ExactActorRef(actor(local)),
                commit_binding: Box::new([]),
                damage: 1,
                health_before: 1,
            },
            position: VersionedPosition {
                actor_local_id: ActorLocalId(local),
                actor_local_generation: ActorLocalGeneration(1),
                context,
                position: LocalPosition {
                    x: 0,
                    y: 0,
                    floor: 7,
                },
                revision: 1,
                facing: None,
            },
        };
        for local in 0..2 {
            carrier.retain_corpse_projection(projection(local));
            carrier
                .death_reward_occurrences
                .push(DeathRewardOccurrence {
                    actor: actor(local),
                    character: [1; 16],
                    occurrence: [2; 16],
                });
        }
        let max = u32::try_from(MAX_RETAINED_CORPSE_PROJECTIONS).expect("bounded");
        for local in 2..=max {
            carrier.retain_corpse_projection(projection(local));
        }
        assert_eq!(
            carrier.corpse_projections.len(),
            MAX_RETAINED_CORPSE_PROJECTIONS
        );
        assert!(
            carrier
                .death_reward_occurrences
                .iter()
                .all(|reward| reward.actor != actor(0))
        );
        assert!(
            carrier
                .death_reward_occurrences
                .iter()
                .any(|reward| reward.actor == actor(1))
        );
    }

    fn corpse_actor_at(continuity: &NamespaceContinuityGuard, local: u32) -> ExactActorRef {
        ExactActorRef(ActorRef {
            world_id: continuity.world_id,
            channel_id: continuity.channel_id,
            scope_generation: continuity.current_generation,
            actor_local_id: ActorLocalId(local),
            actor_local_generation: ActorLocalGeneration(1),
        })
    }

    /// A projected death of local actor `local` in `continuity`'s generation.
    fn corpse_projection_at(
        continuity: &NamespaceContinuityGuard,
        local: u32,
    ) -> RuntimeCorpseProjection {
        RuntimeCorpseProjection {
            death_at_ms: None,
            occurrence: CreatureDeathOccurrenceRef {
                actor: corpse_actor_at(continuity, local),
                commit_binding: Box::new([]),
                damage: 1,
                health_before: 1,
            },
            position: VersionedPosition {
                actor_local_id: ActorLocalId(local),
                actor_local_generation: ActorLocalGeneration(1),
                context: spawn_position_context(continuity),
                position: LocalPosition {
                    x: 0,
                    y: 0,
                    floor: 7,
                },
                revision: 1,
                facing: None,
            },
        }
    }

    /// Codex 4248192466 (LOOT-LIVE-1): a projection pinned by an accepted kill settlement
    /// survives more than 64 newer projections, such as those of kills a full queue refuses,
    /// while the unpinned ones keep their FIFO bound. Once the settle is final and the pin is
    /// released, the next projection evicts it as the oldest unpinned one.
    #[test]
    fn a_pinned_corpse_projection_survives_newer_projections_until_unpinned() {
        let (continuity, mut carrier) = carrier(4);
        let pinned = corpse_actor_at(&continuity, 0);
        carrier.retain_corpse_projection(corpse_projection_at(&continuity, 0));
        carrier
            .death_reward_occurrences
            .push(DeathRewardOccurrence {
                actor: pinned.0,
                character: [1; 16],
                occurrence: [2; 16],
            });
        assert_eq!(carrier.pin_corpse_projection_inner(pinned), Ok(()));
        // Pinning again is a no-op.
        assert_eq!(carrier.pin_corpse_projection_inner(pinned), Ok(()));
        assert_eq!(carrier.corpse_projection_pins, vec![pinned]);
        let max = u32::try_from(MAX_RETAINED_CORPSE_PROJECTIONS).expect("bounded");
        for local in 1..=2 * max {
            carrier.retain_corpse_projection(corpse_projection_at(&continuity, local));
        }
        assert!(carrier.retains_corpse_projection(pinned));
        assert!(
            carrier
                .death_reward_occurrences
                .iter()
                .any(|reward| reward.actor == pinned.0)
        );
        assert_eq!(
            carrier.corpse_projections.len(),
            MAX_RETAINED_CORPSE_PROJECTIONS + 1
        );
        assert!(!carrier.retains_corpse_projection(corpse_actor_at(&continuity, max)));
        assert!(carrier.retains_corpse_projection(corpse_actor_at(&continuity, max + 1)));

        carrier.unpin_corpse_projection_inner(pinned);
        assert!(carrier.corpse_projection_pins.is_empty());
        carrier.retain_corpse_projection(corpse_projection_at(&continuity, 2 * max + 1));
        assert!(!carrier.retains_corpse_projection(pinned));
        assert!(
            carrier
                .death_reward_occurrences
                .iter()
                .all(|reward| reward.actor != pinned.0)
        );
        assert_eq!(
            carrier.corpse_projections.len(),
            MAX_RETAINED_CORPSE_PROJECTIONS
        );
    }

    /// Codex 4248192466: at most `MAX_PINNED_CORPSE_PROJECTIONS` pins, each on a retained
    /// projection of this generation, so `corpse_projections` never holds more than 128.
    #[test]
    fn corpse_projection_pins_are_bounded_and_name_retained_projections() {
        let (continuity, mut carrier) = carrier(4);
        let max = u32::try_from(MAX_PINNED_CORPSE_PROJECTIONS).expect("bounded");
        assert_eq!(
            carrier.pin_corpse_projection_inner(corpse_actor_at(&continuity, 0)),
            Err(CarrierError::CommittedLethalUnavailable),
            "no projection is retained yet"
        );
        for local in 0..max {
            carrier.retain_corpse_projection(corpse_projection_at(&continuity, local));
            assert_eq!(
                carrier.pin_corpse_projection_inner(corpse_actor_at(&continuity, local)),
                Ok(())
            );
        }
        carrier.retain_corpse_projection(corpse_projection_at(&continuity, max));
        assert_eq!(
            carrier.pin_corpse_projection_inner(corpse_actor_at(&continuity, max)),
            Err(CarrierError::CapacityExceeded)
        );
        for local in max + 1..=3 * max {
            carrier.retain_corpse_projection(corpse_projection_at(&continuity, local));
        }
        assert_eq!(
            carrier.corpse_projections.len(),
            MAX_PINNED_CORPSE_PROJECTIONS + MAX_RETAINED_CORPSE_PROJECTIONS
        );
        assert!(
            (0..max).all(|local| carrier.retains_corpse_projection(corpse_actor_at(&continuity, local)))
        );
    }

    /// D115/D116: one spawn, two placement cells, population 2, 60 s respawn / 5 s retry.
    fn d116_definition() -> SpawnDefinition {
        SpawnDefinition::new(
            vec![(1, 0, 7), (2, 0, 7)],
            2,
            60_000_000,
            5_000_000,
            "oteryn:creature/rat".to_string(),
            20,
        )
        .expect("D116 definition is within every registered spawn bound")
    }

    // SPAWN-1a: activation realization and the §6.3 respawn drive on the composed runtime.
    const SPAWN_DELAY: u64 = 60_000_000;
    const SPAWN_RETRY: u64 = 5_000_000;
    const DEN: MovementLocalPosition = MovementLocalPosition {
        x: 2,
        y: 0,
        floor: 7,
    };

    /// The reference 18 x 14 view (MOVE-RL-11) on one floor: x -8..=9, y -6..=7.
    fn reference_view(observer: MovementLocalPosition, cell: MovementLocalPosition) -> bool {
        (-8..=9).contains(&(cell.x - observer.x)) && (-6..=7).contains(&(cell.y - observer.y))
    }

    fn den_definition() -> SpawnDefinition {
        SpawnDefinition::new(
            vec![(2, 0, 7), (2, -1, 7)],
            2,
            SPAWN_DELAY,
            SPAWN_RETRY,
            "oteryn:creature/rat".to_string(),
            20,
        )
        .expect("den definition")
    }

    fn spawned_runtime(definition: SpawnDefinition) -> ChannelRuntimeV1 {
        let mut runtime = runtime(8);
        assert_eq!(runtime.realize_spawn_definitions(vec![definition]), Ok(2));
        runtime
    }

    fn kill_creature(runtime: &mut ChannelRuntimeV1, actor: ExactActorRef) {
        let index = runtime
            .carrier
            .validate_ref(&runtime.continuity, actor.0)
            .expect("live creature");
        let slot = &mut runtime.carrier.slots[index];
        assert!(
            matches!(slot, Slot::CreatureOccupied { .. }),
            "creature slot"
        );
        if let Slot::CreatureOccupied { health, .. } = slot {
            *health = 0;
        }
    }

    fn place_creature(runtime: &mut ChannelRuntimeV1, cell: MovementLocalPosition) {
        let context = runtime.pinned_position_context();
        let actor = runtime
            .carrier
            .admit_creature(&runtime.continuity, ActorState(0), "test:blocker", 20)
            .expect("blocker");
        runtime
            .carrier
            .initialize_position(
                &runtime.continuity,
                actor,
                context,
                LocalPosition {
                    x: cell.x,
                    y: cell.y,
                    floor: cell.floor,
                },
            )
            .expect("blocker position");
    }

    fn place_player(runtime: &mut ChannelRuntimeV1, raw: u64, cell: MovementLocalPosition) {
        let reservation = runtime
            .reserve_fresh_session(session(raw))
            .expect("player reservation");
        let actor = runtime
            .commit_fresh_session(reservation)
            .expect("player commit");
        let context = runtime.pinned_position_context();
        runtime
            .carrier
            .initialize_position(
                &runtime.continuity,
                actor.0,
                context,
                LocalPosition {
                    x: cell.x,
                    y: cell.y,
                    floor: cell.floor,
                },
            )
            .expect("player position");
    }

    fn den_creature(runtime: &ChannelRuntimeV1) -> Option<ExactActorRef> {
        runtime.spawn_point_creatures()[0].2
    }

    #[test]
    fn activation_realizes_one_creature_per_den_cell_with_its_authored_health() {
        let runtime = spawned_runtime(den_definition());
        let points = runtime.spawn_point_creatures();
        assert_eq!(points.len(), 2);
        let census = runtime.positioned_actor_census().expect("census");
        for ((_, cell, live), expected) in points.iter().zip([(2, 0), (2, -1)]) {
            assert_eq!((cell.x, cell.y), expected);
            let actor = live.expect("live rat");
            let (_, snapshot, session) = census
                .iter()
                .find(|(listed, _, _)| *listed == actor)
                .expect("positioned before admission of players");
            assert_eq!(*session, None);
            assert_eq!(snapshot.position(), *cell);
            let index = runtime
                .carrier
                .validate_ref(&runtime.continuity, actor.0)
                .expect("live");
            assert!(matches!(
                runtime.carrier.slots[index],
                Slot::CreatureOccupied { health: 20, .. }
            ));
        }
    }

    #[test]
    fn activation_realizes_once_and_a_restart_realizes_the_same_set() {
        let mut first = spawned_runtime(den_definition());
        assert_eq!(
            first.realize_spawn_definitions(vec![den_definition()]),
            Err(CarrierError::DuplicateSpawnSource)
        );
        let cells = |runtime: &ChannelRuntimeV1| {
            runtime
                .spawn_point_creatures()
                .iter()
                .map(|(point, cell, live)| (*point, *cell, live.is_some()))
                .collect::<Vec<_>>()
        };
        let restarted = spawned_runtime(den_definition());
        assert_eq!(cells(&first), cells(&restarted));
    }

    #[test]
    fn a_dead_rat_warns_one_full_delay_later_and_is_admitted_4200_ms_after_the_warning() {
        let mut runtime = spawned_runtime(den_definition());
        let dead = den_creature(&runtime).expect("live rat");
        kill_creature(&mut runtime, dead);
        // Observed at this drive: the one occurrence is due one full delay later.
        assert_eq!(
            runtime.drive_spawn_respawns(1_000, reference_view),
            Ok(SpawnDriveReport::default())
        );
        let point = runtime.spawn_point_creatures()[0].0;
        assert_eq!(
            runtime.pending_respawn(point).map(|p| p.due_micros),
            Some(1_000 + SPAWN_DELAY)
        );
        // At most one pending occurrence per point: a later drive schedules nothing more.
        runtime
            .drive_spawn_respawns(2_000, reference_view)
            .expect("drive");
        assert_eq!(
            runtime.pending_respawn(point).map(|p| p.due_micros),
            Some(1_000 + SPAWN_DELAY)
        );
        assert_eq!(
            runtime
                .drive_spawn_respawns(SPAWN_DELAY, reference_view)
                .expect("drive"),
            SpawnDriveReport::default()
        );
        let warned = runtime
            .drive_spawn_respawns(1_000 + SPAWN_DELAY, reference_view)
            .expect("drive");
        assert_eq!(warned.warnings, vec![DEN]);
        assert!(warned.admitted.is_empty());
        let admit_at = 1_000 + SPAWN_DELAY + spawn::SPAWN_WARNING_MICROS;
        assert!(
            runtime
                .drive_spawn_respawns(admit_at - 1, reference_view)
                .expect("drive")
                .admitted
                .is_empty()
        );
        let admitted = runtime
            .drive_spawn_respawns(admit_at, reference_view)
            .expect("drive");
        assert_eq!(admitted.admitted.len(), 1);
        let fresh = den_creature(&runtime).expect("respawned rat");
        assert_ne!(fresh, dead);
        assert_eq!(fresh, admitted.admitted[0]);
        assert_eq!(runtime.pending_respawn(point), None);
    }

    #[test]
    fn an_occupied_den_retries_three_times_every_retry_interval_then_skips() {
        let mut runtime = spawned_runtime(den_definition());
        let dead = den_creature(&runtime).expect("live rat");
        kill_creature(&mut runtime, dead);
        runtime
            .drive_spawn_respawns(0, reference_view)
            .expect("observe");
        place_creature(&mut runtime, DEN);
        let mut now = SPAWN_DELAY;
        assert_eq!(
            runtime
                .drive_spawn_respawns(now, reference_view)
                .expect("warn")
                .warnings,
            vec![DEN]
        );
        now += spawn::SPAWN_WARNING_MICROS;
        for _ in 0..spawn::OCCUPANCY_RETRIES_MAX {
            assert_eq!(
                runtime
                    .drive_spawn_respawns(now, reference_view)
                    .expect("occupied")
                    .retried,
                1
            );
            assert!(
                runtime
                    .drive_spawn_respawns(now + SPAWN_RETRY - 1, reference_view)
                    .expect("wait")
                    .warnings
                    .is_empty()
            );
            now += SPAWN_RETRY;
            assert_eq!(
                runtime
                    .drive_spawn_respawns(now, reference_view)
                    .expect("retry")
                    .warnings,
                vec![DEN]
            );
            now += spawn::SPAWN_WARNING_MICROS;
        }
        let skipped = runtime
            .drive_spawn_respawns(now, reference_view)
            .expect("skip");
        assert_eq!((skipped.skipped, skipped.admitted.len()), (1, 0));
        let point = runtime.spawn_point_creatures()[0].0;
        let pending = runtime.pending_respawn(point).expect("successor");
        assert_eq!(
            (pending.due_micros, pending.successor),
            (now + SPAWN_DELAY, 1)
        );
        assert_eq!(den_creature(&runtime), None);
    }

    #[test]
    fn player_interest_blocks_a_blockable_den_and_the_successor_is_a_full_delay_later() {
        let mut runtime = spawned_runtime(den_definition().with_blockable(true));
        let dead = den_creature(&runtime).expect("live rat");
        kill_creature(&mut runtime, dead);
        place_player(
            &mut runtime,
            40,
            MovementLocalPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
        );
        runtime
            .drive_spawn_respawns(0, reference_view)
            .expect("observe");
        let blocked = runtime
            .drive_spawn_respawns(SPAWN_DELAY, reference_view)
            .expect("blocked");
        assert_eq!((blocked.blocked, blocked.warnings.len()), (1, 0));
        let point = runtime.spawn_point_creatures()[0].0;
        assert_eq!(
            runtime.pending_respawn(point).map(|p| p.due_micros),
            Some(2 * SPAWN_DELAY)
        );
    }

    #[test]
    fn player_interest_never_blocks_the_non_blockable_fixture_point() {
        let mut runtime = spawned_runtime(den_definition());
        let dead = den_creature(&runtime).expect("live rat");
        kill_creature(&mut runtime, dead);
        place_player(
            &mut runtime,
            41,
            MovementLocalPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
        );
        runtime
            .drive_spawn_respawns(0, reference_view)
            .expect("observe");
        let warned = runtime
            .drive_spawn_respawns(SPAWN_DELAY, reference_view)
            .expect("warned");
        assert_eq!((warned.blocked, warned.warnings), (0, vec![DEN]));
    }

    #[test]
    fn a_player_on_another_floor_does_not_block_a_blockable_den() {
        let mut runtime = spawned_runtime(den_definition().with_blockable(true));
        let dead = den_creature(&runtime).expect("live rat");
        kill_creature(&mut runtime, dead);
        place_player(
            &mut runtime,
            42,
            MovementLocalPosition {
                x: 2,
                y: 0,
                floor: 6,
            },
        );
        runtime
            .drive_spawn_respawns(0, reference_view)
            .expect("observe");
        let warned = runtime
            .drive_spawn_respawns(SPAWN_DELAY, reference_view)
            .expect("warned");
        assert_eq!((warned.blocked, warned.warnings), (0, vec![DEN]));
    }

    #[test]
    fn realize_spawn_admits_population_creatures_at_declared_cells() {
        let (continuity, mut carrier) = carrier(4);
        let context = spawn_position_context(&continuity);
        carrier
            .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
            .expect("D116 realizes");

        let state_0 = carrier
            .spawn_cell_state(SpawnSourceId(1), 0)
            .expect("cell 0 tracked");
        let state_1 = carrier
            .spawn_cell_state(SpawnSourceId(1), 1)
            .expect("cell 1 tracked");
        let (actor_0, actor_1) = (
            state_0.live.expect("cell 0 has a live rat"),
            state_1.live.expect("cell 1 has a live rat"),
        );
        assert_ne!(actor_0, actor_1);
        assert!(
            carrier
                .current_owner_exact_lookup(&continuity)
                .contains(actor_0)
        );
        assert!(
            carrier
                .current_owner_exact_lookup(&continuity)
                .contains(actor_1)
        );
        assert_eq!(
            carrier
                .read_position(&continuity, actor_0.0)
                .map(|snapshot| snapshot.version.position),
            Ok(LocalPosition {
                x: 1,
                y: 0,
                floor: 7
            })
        );
    }

    #[test]
    fn realize_spawn_rejects_duplicate_source_without_mutation() {
        let (continuity, mut carrier) = carrier(4);
        let context = spawn_position_context(&continuity);
        carrier
            .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
            .expect("first realization");
        let before = carrier.slots.clone();
        assert_eq!(
            carrier.realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context),
            Err(CarrierError::DuplicateSpawnSource)
        );
        assert_eq!(carrier.slots, before);
    }

    #[test]
    fn realize_spawn_with_an_invalid_context_mutates_nothing() {
        // P1 (Codex review, PR #1193): the context is now validated before any admission, so a
        // rejected realization never leaves an untracked, unpositioned creature behind; a
        // retry with a valid context starts from byte-identical state.
        let (continuity, mut carrier) = carrier(4);
        let mut invalid_context = spawn_position_context(&continuity);
        invalid_context.coordinate_frame_marker = 0;
        let slots_before = carrier.slots.clone();
        let free_head_before = carrier.free_head;
        assert_eq!(
            carrier.realize_spawn(
                &continuity,
                SpawnSourceId(1),
                d116_definition(),
                invalid_context
            ),
            Err(CarrierError::InvalidPreProductionPositionContext)
        );
        assert_eq!(carrier.slots, slots_before);
        assert_eq!(carrier.free_head, free_head_before);
        assert!(carrier.spawns.is_empty());

        let context = spawn_position_context(&continuity);
        carrier
            .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
            .expect("a valid context still realizes the spawn afterward");
    }

    /// A one-source definition with `points` cells on one row, starting at x = `first_x`.
    fn row_definition(first_x: i32, points: usize) -> SpawnDefinition {
        let cells = (0..points)
            .map(|offset| (first_x + i32::try_from(offset).expect("small"), 0, 7))
            .collect::<Vec<_>>();
        SpawnDefinition::new(
            cells,
            points,
            60_000_000,
            5_000_000,
            "oteryn:creature/rat".to_string(),
            20,
        )
        .expect("within CREATUREAI0-RL-03")
    }

    /// `CREATUREAI0-RL-03`: 64 points (one cell and one creature each) per source pass and
    /// 65 refuse, whether counted by cells or by population.
    #[test]
    fn spawn_points_per_source_max_accepted_max_plus_one_rejected() {
        assert_eq!(CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX, 64);
        let cells = |count: usize| {
            (0..count)
                .map(|x| (i32::try_from(x).expect("small"), 0, 0))
                .collect::<Vec<(i32, i32, i16)>>()
        };
        let definition = |cells, population| {
            SpawnDefinition::new(
                cells,
                population,
                60_000_000,
                5_000_000,
                "oteryn:creature/rat".to_string(),
                20,
            )
        };
        let max = CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
        assert!(definition(cells(max), max).is_ok());
        assert_eq!(
            definition(cells(max + 1), max + 1),
            Err(CarrierError::InvalidSpawnDefinition)
        );
        assert_eq!(
            definition(cells(max), max + 1),
            Err(CarrierError::InvalidSpawnDefinition)
        );
        assert_eq!(
            definition(cells(max + 1), max),
            Err(CarrierError::InvalidSpawnDefinition)
        );
        // The former first-slice bound (4) no longer limits a source.
        let mut runtime = runtime(2 * max);
        assert_eq!(
            runtime.realize_spawn_definitions(vec![row_definition(0, max)]),
            Ok(max)
        );
        assert_eq!(runtime.spawn_point_creatures().len(), max);
    }

    /// `CREATUREAI0-RL-01`: the channel ceiling on spawn points is checked as a sum, not as
    /// the product of the per-source and source-count rows. 131,072 points pass; one more
    /// refuses; so does more than `CREATUREAI0-RL-02` sources, whatever their points.
    #[test]
    fn spawn_points_per_channel_ceiling_is_exact() {
        assert_eq!(CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX, 131_072);
        let max = CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX;
        let per_source = CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
        let mut definitions = (0..max / per_source)
            .map(|_| row_definition(0, per_source))
            .collect::<Vec<_>>();
        assert_eq!(spawn_points_within_ceilings(&definitions), Ok(max));
        definitions.push(row_definition(0, 1));
        assert_eq!(
            spawn_points_within_ceilings(&definitions),
            Err(CarrierError::CapacityExceeded)
        );
        let one_point_sources =
            vec![row_definition(0, 1); CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX];
        assert_eq!(
            spawn_points_within_ceilings(&one_point_sources),
            Ok(CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX)
        );
        let mut over_sources = one_point_sources;
        over_sources.push(row_definition(0, 1));
        assert_eq!(
            spawn_points_within_ceilings(&over_sources),
            Err(CarrierError::CapacityExceeded)
        );

        // At the runtime, one point over refuses before any admission or reservation, even
        // with free slots to spare.
        let mut runtime = runtime(8);
        let slots_before = runtime.carrier.slots.clone();
        assert_eq!(
            runtime.realize_spawn_definitions(definitions),
            Err(CarrierError::CapacityExceeded)
        );
        assert_eq!(runtime.carrier.slots, slots_before);
        assert!(runtime.carrier.spawns.is_empty());
        assert!(runtime.carrier.occupied.is_empty());
    }

    /// `CREATUREAI0-RL-02` at the runtime: 65,536 one-point sources realize in canonical
    /// order (the former 16-source ceiling no longer refuses readiness); 65,537 refuse with
    /// nothing admitted.
    #[test]
    fn spawn_sources_per_channel_max_accepted_max_plus_one_rejected() {
        assert_eq!(CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX, 65_536);
        let max = CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX;
        let sources = |count: usize| {
            (0..count)
                .map(|index| row_definition(i32::try_from(index % 256).expect("small"), 1))
                .collect::<Vec<_>>()
        };

        let mut over = runtime(max + 1);
        let slots_before = over.carrier.slots.clone();
        assert_eq!(
            over.realize_spawn_definitions(sources(max + 1)),
            Err(CarrierError::CapacityExceeded)
        );
        assert_eq!(over.carrier.slots, slots_before);
        assert!(over.carrier.spawns.is_empty());
        assert!(over.carrier.occupied.is_empty());
        drop(over);

        let mut runtime = runtime(max + SPAWN_ACTIVATION_PLAYER_HEADROOM);
        // More than one owner window: one turn refuses, owner windows realize it.
        assert_eq!(
            runtime.realize_spawn_definitions(sources(max)),
            Err(CarrierError::CapacityExceeded)
        );
        assert!(runtime.carrier.spawns.is_empty());
        assert_eq!(
            realize_in_owner_windows(&mut runtime, sources(max)),
            (max, max / spawn::REALIZATIONS_PER_WINDOW_MAX)
        );
        assert_eq!(runtime.carrier.spawns.len(), max);
        assert!(
            runtime
                .carrier
                .spawns
                .windows(2)
                .all(|pair| pair[0].source.0 < pair[1].source.0)
        );
        let last = SpawnSourceId(u16::MAX);
        assert!(runtime.carrier.spawn_cell_state(last, 0).is_ok());
        // Every source ordinal is taken: a further single source is a duplicate, and nothing
        // changes.
        let context = runtime.pinned_position_context();
        assert_eq!(
            runtime
                .carrier
                .realize_spawn(&runtime.continuity, last, row_definition(0, 1), context),
            Err(CarrierError::DuplicateSpawnSource)
        );
        assert_eq!(runtime.carrier.spawns.len(), max);
    }

    /// The creature key and health of live creature `actor`.
    fn spawned_creature(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Option<(Vec<u8>, i64)> {
        let index = runtime
            .carrier
            .validate_ref(&runtime.continuity, actor.0)
            .ok()?;
        match &runtime.carrier.slots[index] {
            Slot::CreatureOccupied {
                target_identity,
                health,
                ..
            } => Some((target_identity.to_vec(), *health)),
            _ => None,
        }
    }

    /// SPAWN-1b (Codex P1 on #1981): one authored source whose points mix creatures, respawn
    /// delays and initial health stays one runtime source. Its point ordinals run 0..3 in
    /// authored order without a reset, each point realizes its own creature with its own
    /// health, and each respawns after its own delay as its own creature.
    #[test]
    fn spawn1b_mixed_point_source_is_one_runtime_source_with_per_point_facts() {
        const RAT: &str = "oteryn:creature/rat";
        const BAT: &str = "oteryn:creature/bat";
        let rat = || Some((RAT.as_bytes().to_vec(), 20));
        let bat = || Some((BAT.as_bytes().to_vec(), 35));
        let mut runtime = runtime(8);
        let source: ActivationSpawnSourceFacts = (
            vec![
                ((2, 0, 7), 60_000, RAT.to_string(), 20, false),
                ((3, 0, 7), 30_000, BAT.to_string(), 35, false),
                ((4, 0, 7), 30_000, RAT.to_string(), 20, false),
            ],
            5_000,
        );
        assert_eq!(runtime.begin_activation_spawn_sources(vec![source]), Ok(3));
        let progress = runtime
            .realize_activation_spawn_window(0)
            .expect("one window");
        assert_eq!((progress.realized, progress.remaining), (3, 0));
        assert_eq!(runtime.carrier.spawns.len(), 1);
        let points = runtime.spawn_point_creatures();
        assert_eq!(
            points
                .iter()
                .map(|(point, cell, _)| (point.source, point.ordinal, cell.x))
                .collect::<Vec<_>>(),
            vec![(0, 0, 2), (0, 1, 3), (0, 2, 4)]
        );
        let live = points
            .iter()
            .map(|(_, _, live)| live.expect("live creature"))
            .collect::<Vec<_>>();
        assert_eq!(
            live.iter()
                .map(|actor| spawned_creature(&runtime, *actor))
                .collect::<Vec<_>>(),
            vec![rat(), bat(), rat()]
        );

        for actor in &live {
            kill_creature(&mut runtime, *actor);
        }
        runtime
            .drive_spawn_respawns(1_000, reference_view)
            .expect("observe");
        assert_eq!(
            points
                .iter()
                .map(|(point, _, _)| runtime.pending_respawn(*point).map(|p| p.due_micros))
                .collect::<Vec<_>>(),
            vec![
                Some(1_000 + 60_000_000),
                Some(1_000 + 30_000_000),
                Some(1_000 + 30_000_000)
            ]
        );
        let warned = runtime
            .drive_spawn_respawns(1_000 + 30_000_000, reference_view)
            .expect("warn");
        assert_eq!(warned.warnings.len(), 2);
        let admitted = runtime
            .drive_spawn_respawns(
                1_000 + 30_000_000 + spawn::SPAWN_WARNING_MICROS,
                reference_view,
            )
            .expect("admit");
        assert_eq!(
            admitted
                .admitted
                .iter()
                .map(|actor| spawned_creature(&runtime, *actor))
                .collect::<Vec<_>>(),
            vec![bat(), rat()]
        );
        assert_eq!(
            runtime
                .pending_respawn(points[0].0)
                .map(|pending| pending.due_micros),
            Some(1_000 + 60_000_000)
        );
    }

    /// SPAWN-1b (Codex P2 on #2002): blockability is per point (CREATURE-AI-0 §6.3, the point's
    /// creature profile). In one source mixing a blockable rat and a non-blockable bat, a
    /// player whose interest area holds both cells blocks the rat's respawn (`BLOCKED`, a
    /// successor a full delay later) while the bat's warns and admits as usual.
    #[test]
    fn spawn1b_mixed_source_blocks_only_its_blockable_points() {
        const RAT: &str = "oteryn:creature/rat";
        const BAT: &str = "oteryn:creature/bat";
        const BAT_CELL: MovementLocalPosition = MovementLocalPosition {
            x: 3,
            y: 0,
            floor: 7,
        };
        let source: ActivationSpawnSourceFacts = (
            vec![
                ((2, 0, 7), 60_000, RAT.to_string(), 20, true),
                ((3, 0, 7), 60_000, BAT.to_string(), 35, false),
            ],
            5_000,
        );
        let definition =
            SpawnDefinition::from_activation_source(source.clone()).expect("valid source");
        assert_eq!(
            [0, 1].map(|ordinal| definition.timing(ordinal).map(|timing| timing.blockable)),
            [Ok(true), Ok(false)]
        );

        let mut runtime = runtime(8);
        assert_eq!(runtime.begin_activation_spawn_sources(vec![source]), Ok(2));
        let progress = runtime
            .realize_activation_spawn_window(0)
            .expect("one window");
        assert_eq!(progress.remaining, 0);
        let points = runtime.spawn_point_creatures();
        for (_, _, live) in &points {
            kill_creature(&mut runtime, live.expect("live creature"));
        }
        place_player(
            &mut runtime,
            43,
            MovementLocalPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
        );
        runtime
            .drive_spawn_respawns(0, reference_view)
            .expect("observe");
        let due = runtime
            .drive_spawn_respawns(SPAWN_DELAY, reference_view)
            .expect("due");
        assert_eq!((due.blocked, due.warnings), (1, vec![BAT_CELL]));
        assert_eq!(
            runtime.pending_respawn(points[0].0).map(|p| p.due_micros),
            Some(2 * SPAWN_DELAY)
        );
        let admitted = runtime
            .drive_spawn_respawns(SPAWN_DELAY + spawn::SPAWN_WARNING_MICROS, reference_view)
            .expect("admit");
        assert_eq!(
            admitted
                .admitted
                .iter()
                .map(|actor| spawned_creature(&runtime, *actor))
                .collect::<Vec<_>>(),
            vec![Some((BAT.as_bytes().to_vec(), 35))]
        );
        assert_eq!(
            runtime.pending_respawn(points[0].0).map(|p| p.due_micros),
            Some(2 * SPAWN_DELAY)
        );
    }

    /// SPAWN-1b: a mixed source keeps every per-source check: `CREATUREAI0-RL-03` (64 points
    /// pass, 65 refuse), a positive health on every point and no empty source. A refused
    /// source refuses the whole activation with nothing begun.
    #[test]
    fn spawn1b_mixed_point_source_keeps_the_per_source_checks() {
        let points = |count: usize, health: u64| {
            (0..count)
                .map(|x| {
                    let key = if x % 2 == 0 {
                        "oteryn:creature/rat"
                    } else {
                        "oteryn:creature/bat"
                    };
                    (
                        (i32::try_from(x).expect("small"), 0, 7),
                        60_000,
                        key.to_string(),
                        health,
                        false,
                    )
                })
                .collect::<Vec<_>>()
        };
        let max = CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
        assert!(SpawnDefinition::from_activation_source((points(max, 20), 5_000)).is_ok());
        for refused in [
            (points(max + 1, 20), 5_000),
            (points(0, 20), 5_000),
            (points(2, 0), 5_000),
            (
                vec![(
                    (0, 0, 7),
                    u64::MAX,
                    "oteryn:creature/rat".to_string(),
                    20,
                    false,
                )],
                5_000,
            ),
            (points(1, u64::MAX), 5_000),
        ] {
            assert_eq!(
                SpawnDefinition::from_activation_source(refused.clone()),
                Err(CarrierError::InvalidSpawnDefinition)
            );
            let mut runtime = runtime(256);
            assert_eq!(
                runtime.begin_activation_spawn_sources(vec![(points(1, 20), 5_000), refused]),
                Err(CarrierError::InvalidSpawnDefinition)
            );
            assert!(runtime.spawn_activation.is_none());
            assert!(runtime.carrier.spawns.is_empty());
        }
    }

    /// SPAWN-1b (Codex P1 on #1981): the pinned preprod bundle's scale, 51,078 authored sources
    /// with 82,673 active points of mixed creatures and delays, is 51,078 runtime sources: it
    /// passes `CREATUREAI0-RL-02` and `CREATUREAI0-RL-01` and realizes window by window. Split
    /// into its 70,206 creature/delay runs it would refuse `CapacityExceeded`, and so does
    /// 65,537 sources through the per-point entry.
    #[test]
    fn spawn1b_bundle_scale_sources_stay_under_the_source_ceiling() {
        const SOURCES: usize = 51_078;
        const POINTS: usize = 82_673;
        const SPLIT_RUNS: usize = 70_206;
        // 31,595 sources of a rat and a bat with different delays, the rest one rat each.
        let mixed = POINTS - SOURCES;
        let source = |index: usize, two: bool| -> ActivationSpawnSourceFacts {
            let x = i32::try_from(index % 256).expect("small");
            let y = i32::try_from(index / 256).expect("small");
            let mut points = vec![(
                (x, y, 7),
                60_000,
                "oteryn:creature/rat".to_string(),
                20,
                false,
            )];
            if two {
                points.push((
                    (x, y + 512, 7),
                    30_000,
                    "oteryn:creature/bat".to_string(),
                    35,
                    false,
                ));
            }
            (points, 5_000)
        };
        let sources = (0..SOURCES)
            .map(|index| source(index, index < mixed))
            .collect::<Vec<_>>();
        let definitions = sources
            .iter()
            .cloned()
            .map(SpawnDefinition::from_activation_source)
            .collect::<Result<Vec<_>, _>>()
            .expect("valid sources");
        assert_eq!(definitions.len(), SOURCES);
        assert_eq!(spawn_points_within_ceilings(&definitions), Ok(POINTS));
        let split_runs = vec![row_definition(0, 1); SPLIT_RUNS];
        assert_eq!(
            spawn_points_within_ceilings(&split_runs),
            Err(CarrierError::CapacityExceeded)
        );
        drop(split_runs);

        let mut over = runtime(8);
        assert_eq!(
            over.begin_activation_spawn_sources(
                (0..=CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX)
                    .map(|index| source(index, false))
                    .collect()
            ),
            Err(CarrierError::CapacityExceeded)
        );
        assert!(over.spawn_activation.is_none());
        drop(over);

        let mut runtime = runtime(POINTS + SPAWN_ACTIVATION_PLAYER_HEADROOM);
        assert_eq!(runtime.begin_activation_spawn_sources(sources), Ok(POINTS));
        let mut now = 0;
        loop {
            let progress = runtime
                .realize_activation_spawn_window(now)
                .expect("one window");
            if progress.remaining == 0 {
                assert_eq!(progress.realized_total, POINTS);
                break;
            }
            now += spawn::OWNER_WINDOW_MICROS;
        }
        assert_eq!(runtime.carrier.spawns.len(), SOURCES);
        let points = runtime.spawn_point_creatures();
        assert_eq!(points.len(), POINTS);
        let first = points
            .iter()
            .take(2)
            .map(|(point, _, live)| {
                (
                    (point.source, point.ordinal),
                    spawned_creature(&runtime, live.expect("live")),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            first,
            vec![
                ((0, 0), Some((b"oteryn:creature/rat".to_vec(), 20))),
                ((0, 1), Some((b"oteryn:creature/bat".to_vec(), 35))),
            ]
        );
    }

    /// Runs a begun activation one window per owner window from owner time 0, checking that no
    /// owner window realizes more than `CREATUREAI0-RL-14` points and that a second call in an
    /// owner window realizes nothing. Returns the creatures realized and the windows used.
    fn realize_in_owner_windows(
        runtime: &mut ChannelRuntimeV1,
        definitions: Vec<SpawnDefinition>,
    ) -> (usize, usize) {
        let total = runtime.begin_spawn_definitions(definitions).expect("begin");
        let mut now = 0;
        let mut windows = 0;
        loop {
            let progress = runtime
                .realize_activation_spawn_window(now)
                .expect("one window");
            assert!(progress.realized <= spawn::REALIZATIONS_PER_WINDOW_MAX);
            windows += 1;
            if progress.remaining == 0 {
                assert_eq!(progress.realized_total, total);
                assert!(runtime.spawn_activation.is_none());
                return (total, windows);
            }
            let again = runtime
                .realize_activation_spawn_window(now + spawn::OWNER_WINDOW_MICROS - 1)
                .expect("same owner window");
            assert_eq!(again.realized, 0);
            assert_eq!(again.remaining, progress.remaining);
            now += spawn::OWNER_WINDOW_MICROS;
        }
    }

    /// SPAWN-CEILINGS-1 (Codex 4242094682): `CREATUREAI0-RL-14` holds above one window. One
    /// owner turn refuses an activation of more than 4,096 points with nothing realized; begun
    /// instead, it realizes exactly the `spawn::realization_windows` plan, one window per owner
    /// window, in canonical order, a source spanning two windows included, and the respawn
    /// drive does nothing until it completes.
    #[test]
    fn activation_realizes_at_most_rl_14_points_per_owner_window() {
        // 131 sources of 63 points: 8,253 points, windows of 4,096, 4,096 and 61; source 65
        // has 1 point in the first window and 62 in the second.
        let definitions = || (0..131).map(|_| row_definition(0, 63)).collect::<Vec<_>>();
        let mut runtime = runtime(8_253 + SPAWN_ACTIVATION_PLAYER_HEADROOM);
        let slots_before = runtime.carrier.slots.clone();
        assert_eq!(
            runtime.realize_spawn_definitions(definitions()),
            Err(CarrierError::CapacityExceeded)
        );
        assert_eq!(runtime.carrier.slots, slots_before);
        assert!(runtime.carrier.spawns.is_empty() && runtime.spawn_activation.is_none());

        assert_eq!(runtime.begin_spawn_definitions(definitions()), Ok(8_253));
        assert_eq!(runtime.carrier.slots, slots_before, "begin admits nothing");
        assert_eq!(
            runtime.begin_spawn_definitions(definitions()),
            Err(CarrierError::DuplicateSpawnSource)
        );
        let plan = spawn::realization_windows(
            &(0..131_u16)
                .flat_map(|source| {
                    (0..63_u8).map(move |ordinal| spawn::SpawnPoint { source, ordinal })
                })
                .collect::<Vec<_>>(),
        )
        .iter()
        .map(Vec::len)
        .collect::<Vec<_>>();
        assert_eq!(plan, vec![4_096, 4_096, 61]);

        let first = runtime
            .realize_activation_spawn_window(1_000)
            .expect("first window");
        assert_eq!((first.realized, first.remaining), (4_096, 4_157));
        assert_eq!(runtime.spawn_point_creatures().len(), 4_096);
        assert_eq!(runtime.carrier.spawns[65].cells.len(), 1);
        // Nothing respawns or realizes outside the plan while the activation runs.
        assert_eq!(
            runtime.drive_spawn_respawns(1_000, reference_view),
            Ok(SpawnDriveReport::default())
        );
        let same = runtime
            .realize_activation_spawn_window(spawn::OWNER_WINDOW_MICROS - 1)
            .expect("same owner window");
        assert_eq!((same.realized, same.remaining), (0, 4_157));
        assert_eq!(runtime.spawn_point_creatures().len(), 4_096);

        let second = runtime
            .realize_activation_spawn_window(spawn::OWNER_WINDOW_MICROS)
            .expect("second window");
        assert_eq!((second.realized, second.remaining), (4_096, 61));
        assert_eq!(runtime.carrier.spawns[65].cells.len(), 63);
        let third = runtime
            .realize_activation_spawn_window(2 * spawn::OWNER_WINDOW_MICROS)
            .expect("last window");
        assert_eq!(
            (third.realized, third.realized_total, third.remaining),
            (61, 8_253, 0)
        );
        assert!(runtime.spawn_activation.is_none());
        let points = runtime.spawn_point_creatures();
        assert_eq!(points.len(), 8_253);
        assert!(points.iter().all(|(_, _, live)| live.is_some()));
        assert!(points.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert_eq!(
            runtime.realize_activation_spawn_window(3 * spawn::OWNER_WINDOW_MICROS),
            Err(CarrierError::PlanConflict)
        );
    }

    /// A refusal in a later activation window realizes nothing: the earlier windows'
    /// creatures are removed, other actors admitted between windows stay, and a new
    /// activation may begin.
    #[test]
    fn a_refused_activation_window_realizes_nothing() {
        // 65 sources of 64 points: windows of 4,096 and 64.
        let capacity = 4_160 + SPAWN_ACTIVATION_PLAYER_HEADROOM;
        let mut runtime = runtime(capacity);
        assert_eq!(
            runtime.begin_spawn_definitions((0..65).map(|_| row_definition(0, 64)).collect()),
            Ok(4_160)
        );
        let first = runtime.realize_activation_spawn_window(0).expect("first");
        assert_eq!(first.realized, 4_096);
        let early = runtime.spawn_point_creatures()[0].2.expect("realized");
        // Other owner work takes slots between windows: the second window admits 63 of its 64
        // points and then refuses.
        let others = (0..runtime.carrier.free_slot_count() - 63)
            .map(|_| {
                runtime
                    .carrier
                    .admit_creature(&runtime.continuity, ActorState(0), "test:other", 20)
                    .expect("other creature")
            })
            .collect::<Vec<_>>();
        assert_eq!(others.len(), SPAWN_ACTIVATION_PLAYER_HEADROOM + 1);
        assert_eq!(
            runtime.realize_activation_spawn_window(spawn::OWNER_WINDOW_MICROS),
            Err(CarrierError::CapacityExceeded)
        );
        assert!(runtime.carrier.spawns.is_empty() && runtime.spawn_activation.is_none());
        assert!(!runtime.carrier.creature_alive(&runtime.continuity, early));
        for other in &others {
            assert!(
                runtime
                    .carrier
                    .creature_alive(&runtime.continuity, ExactActorRef(*other))
            );
        }
        assert_eq!(runtime.carrier.occupied.len(), others.len());
        assert_eq!(runtime.carrier.free_slot_count(), capacity - others.len());
        assert_eq!(
            runtime.realize_spawn_definitions(vec![row_definition(0, 64)]),
            Ok(64)
        );
    }

    /// SPAWN-CEILINGS-1 (Codex 4242245064): an activation leaves
    /// [`SPAWN_ACTIVATION_PLAYER_HEADROOM`] slots of the pre-production carrier free, so players
    /// are admitted after the largest accepted activation; filling the carrier with
    /// `CREATUREAI0-RL-01` points refuses with nothing kept.
    #[test]
    fn activation_keeps_player_headroom_in_the_pre_production_carrier() {
        // The pre-production carrier capacity (`PREPRODUCTION_FIRST_SLICE_ACTOR_CAPACITY`,
        // 131,072) equals `CREATUREAI0-RL-01`; this file is also built into crates without
        // the node configuration, so the test names the equal ceiling.
        let capacity = CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX;
        assert_eq!(
            spawn_activation_player_headroom(capacity),
            SPAWN_ACTIVATION_PLAYER_HEADROOM
        );
        let points = |total: usize| {
            let mut definitions = (0..total / 64)
                .map(|index| row_definition(i32::try_from(index % 4).expect("small") * 64, 64))
                .collect::<Vec<_>>();
            if !total.is_multiple_of(64) {
                definitions.push(row_definition(0, total % 64));
            }
            definitions
        };
        let mut runtime = runtime(capacity);
        let largest = capacity - SPAWN_ACTIVATION_PLAYER_HEADROOM;
        for refused in [capacity, largest + 1] {
            assert_eq!(
                runtime.begin_spawn_definitions(points(refused)),
                Err(CarrierError::CapacityExceeded)
            );
            assert!(runtime.spawn_activation.is_none() && runtime.carrier.spawns.is_empty());
        }
        assert_eq!(
            realize_in_owner_windows(&mut runtime, points(largest)).0,
            largest
        );
        for raw in 0..u64::try_from(SPAWN_ACTIVATION_PLAYER_HEADROOM).expect("small") {
            let reservation = runtime
                .reserve_fresh_session(session(1_000 + raw))
                .expect("player headroom");
            runtime
                .commit_fresh_session(reservation)
                .expect("player commit");
        }
        assert_eq!(runtime.carrier.free_slot_count(), 0);
    }

    /// SPAWN-CEILINGS-1 (Codex 4242381779): `CREATUREAI0-RL-04` is the creature admission
    /// ceiling. The pre-production carrier is smaller, so it never binds there; a carrier with
    /// one slot more than the ceiling admits 262,144 creatures, refuses the next one with
    /// nothing written and still admits a player into the free slot.
    #[test]
    fn creature_admission_refuses_past_the_creature_actor_ceiling() {
        const {
            assert!(
                CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX
                    < CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
            );
        }
        let mut runtime = runtime(CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX + 1);
        for _ in 0..CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX {
            runtime
                .carrier
                .admit_creature(&runtime.continuity, ActorState(0), "test:creature", 20)
                .expect("below the ceiling");
        }
        assert_eq!(
            runtime
                .carrier
                .admit_creature(&runtime.continuity, ActorState(0), "test:creature", 20),
            Err(CarrierError::CapacityExceeded)
        );
        assert_eq!(runtime.carrier.free_slot_count(), 1);
        let reservation = runtime
            .reserve_fresh_session(session(1))
            .expect("the free slot admits a player");
        runtime
            .commit_fresh_session(reservation)
            .expect("player commit");
        assert_eq!(runtime.carrier.free_slot_count(), 0);
    }

    /// SPAWN-CEILINGS-1 (Codex 4242094686): the full spawn scan runs once per Channel owner
    /// window, not once per drive. The owner drives once per connected session; a creature
    /// that dies after a window's first drive is observed only at the next window's first
    /// drive, however many sessions drive meanwhile, and an occurrence due inside a window
    /// still runs at its exact deadline.
    #[test]
    fn the_spawn_scan_runs_once_per_owner_window_however_many_sessions_drive() {
        let mut runtime = spawned_runtime(den_definition());
        let point = runtime.spawn_point_creatures()[0].0;
        assert_eq!(
            runtime.drive_spawn_respawns(0, reference_view),
            Ok(SpawnDriveReport::default())
        );
        let dead = den_creature(&runtime).expect("live rat");
        kill_creature(&mut runtime, dead);
        for session_drive in 1..=256 {
            assert_eq!(
                runtime.drive_spawn_respawns(session_drive, reference_view),
                Ok(SpawnDriveReport::default())
            );
        }
        assert_eq!(
            runtime.pending_respawn(point),
            None,
            "no rescan in the window"
        );
        let observed_at = spawn::OWNER_WINDOW_MICROS + 123;
        runtime
            .drive_spawn_respawns(observed_at, reference_view)
            .expect("next window");
        let due = observed_at + SPAWN_DELAY;
        assert_eq!(
            runtime.pending_respawn(point).map(|p| p.due_micros),
            Some(due)
        );
        // The window holding the deadline: its drives before the deadline run nothing, the
        // drive at the deadline warns.
        let window_start = due - 123;
        for now in window_start..due {
            assert_eq!(
                runtime.drive_spawn_respawns(now, reference_view),
                Ok(SpawnDriveReport::default())
            );
        }
        let warned = runtime
            .drive_spawn_respawns(due, reference_view)
            .expect("deadline");
        assert_eq!(warned.warnings, vec![DEN]);
        assert_eq!(
            runtime.drive_spawn_respawns(due + 1, reference_view),
            Ok(SpawnDriveReport::default())
        );
    }

    /// SPAWN-CEILINGS-1 (Codex 4242270322): when many respawns fall due at once, each owner
    /// window runs at most [`SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX`] steps however many drives it
    /// gets; the rest stay pending, run first in the next windows, and every point is admitted
    /// again in the end.
    #[test]
    fn simultaneous_respawns_run_a_bounded_number_of_steps_per_owner_window() {
        let sources = 4;
        let total = sources * CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
        let mut runtime = runtime(2 * total);
        let definitions = (0..sources)
            .map(|source| {
                row_definition(
                    i32::try_from(source * CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX).expect("small"),
                    CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX,
                )
            })
            .collect();
        assert_eq!(runtime.realize_spawn_definitions(definitions), Ok(total));
        let points = runtime.spawn_point_creatures();
        assert_eq!(points.len(), total);
        for (_, _, live) in &points {
            kill_creature(&mut runtime, live.expect("realized creature"));
        }
        // One drive observes every death: all occurrences fall due at the same deadline.
        assert_eq!(
            runtime.drive_spawn_respawns(0, reference_view),
            Ok(SpawnDriveReport::default())
        );
        assert!(points.iter().all(|(point, _, _)| {
            runtime.pending_respawn(*point).map(|p| p.due_micros) == Some(SPAWN_DELAY)
        }));
        let steps = |report: &SpawnDriveReport| {
            report.warnings.len()
                + report.admitted.len()
                + report.blocked
                + report.retried
                + report.skipped
        };
        let first = runtime
            .drive_spawn_respawns(SPAWN_DELAY, reference_view)
            .expect("deadline");
        assert_eq!(first.warnings.len(), SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX);
        // Later drives of the same window (one per session) run nothing more.
        for session_drive in 1..=64 {
            assert_eq!(
                runtime.drive_spawn_respawns(SPAWN_DELAY + session_drive, reference_view),
                Ok(SpawnDriveReport::default())
            );
        }
        let mut admitted = 0;
        let mut warned = first.warnings.len();
        let mut window_start = SPAWN_DELAY - SPAWN_DELAY % spawn::OWNER_WINDOW_MICROS;
        let mut windows = 0;
        while admitted < total {
            window_start += spawn::OWNER_WINDOW_MICROS;
            windows += 1;
            assert!(windows < 10_000, "every respawn completes");
            let mut window_steps = 0;
            for offset in [0, 1, spawn::OWNER_WINDOW_MICROS - 1] {
                let report = runtime
                    .drive_spawn_respawns(window_start + offset, reference_view)
                    .expect("drive");
                assert_eq!(report.blocked + report.retried + report.skipped, 0);
                window_steps += steps(&report);
                warned += report.warnings.len();
                admitted += report.admitted.len();
            }
            assert!(window_steps <= SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX);
        }
        assert_eq!((warned, admitted), (total, total));
        for (point, cell, live) in runtime.spawn_point_creatures() {
            assert_eq!(runtime.pending_respawn(point), None);
            let fresh = live.expect("respawned creature");
            assert!(runtime.carrier.creature_alive(&runtime.continuity, fresh));
            assert!(!points.iter().any(|(_, _, dead)| *dead == Some(fresh)));
            assert_eq!(
                points
                    .iter()
                    .find(|(dead_point, _, _)| *dead_point == point)
                    .map(|(_, dead_cell, _)| *dead_cell),
                Some(cell)
            );
        }
        // The dead creatures are gone: the occupied index holds exactly the respawned ones.
        assert_eq!(runtime.carrier.occupied.len(), total);
    }

    /// SPAWN-CEILINGS-1 (Codex 4243397472): with all 131,072 creatures of a Channel dead at once,
    /// each owner window reads at most [`SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX`] points and
    /// [`SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX`] deadline entries, however many drives it gets:
    /// the gone scan observes the deaths 4,096 per window from its cursor, and once the whole
    /// backlog is due each window runs its 32 steps by deadline then point without collecting
    /// or sorting the backlog. The deadline index equals the schedule after every window.
    #[test]
    fn a_131_072_death_backlog_reads_bounded_spawn_records_per_owner_window() {
        let total = CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX;
        let per_source = CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
        let mut runtime = runtime(total + SPAWN_ACTIVATION_PLAYER_HEADROOM);
        let definitions = (0..total / per_source)
            .map(|_| row_definition(0, per_source))
            .collect();
        let (realized, activation_windows) = realize_in_owner_windows(&mut runtime, definitions);
        assert_eq!(realized, total);
        let points = runtime.spawn_point_creatures();
        for (_, _, live) in &points {
            kill_creature(&mut runtime, live.expect("realized creature"));
        }
        let window_reads =
            SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX + SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX;
        let window_micros = spawn::OWNER_WINDOW_MICROS;
        let start = u64::try_from(activation_windows).expect("small") * window_micros;
        let drive_window = |runtime: &mut ChannelRuntimeV1, now: u64| {
            let report = runtime
                .drive_spawn_respawns(now, reference_view)
                .expect("drive");
            // Another session's drive in the same window runs nothing more.
            assert_eq!(
                runtime.drive_spawn_respawns(now + 1, reference_view),
                Ok(SpawnDriveReport::default())
            );
            let (visits, consistent) = runtime.take_spawn_drive_visits();
            assert!(consistent, "the deadline index equals the schedule");
            assert!(
                visits <= window_reads,
                "{visits} records read in one window"
            );
            report
        };
        // The gone scan: 4,096 deaths observed per window, 32 windows for the whole Channel.
        let scan_windows = total / SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX;
        assert_eq!(scan_windows, 32);
        let _ = runtime.take_spawn_drive_visits();
        for window in 0..scan_windows {
            let now = start + u64::try_from(window).expect("small") * window_micros;
            assert_eq!(drive_window(&mut runtime, now), SpawnDriveReport::default());
            assert_eq!(
                runtime.spawn_schedule.len(),
                (window + 1) * SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX
            );
        }
        for (index, (point, _, _)) in points.iter().enumerate() {
            let window =
                u64::try_from(index / SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX).expect("small");
            assert_eq!(
                runtime
                    .pending_respawn(*point)
                    .map(|pending| pending.due_micros),
                Some(start + window * window_micros + SPAWN_DELAY)
            );
        }
        // The whole backlog is due: each window warns 32 points, oldest deadline first.
        let mut now =
            start + u64::try_from(scan_windows).expect("small") * window_micros + SPAWN_DELAY;
        for window in 0..64 {
            let report = drive_window(&mut runtime, now);
            assert_eq!(report.warnings.len(), SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX);
            assert!(report.admitted.is_empty());
            now += window_micros;
            let warned = (window + 1) * SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX;
            assert_eq!(
                runtime
                    .pending_respawn(points[warned - 1].0)
                    .map(|pending| pending.step),
                Some(spawn::RespawnStep::Admit)
            );
            assert_eq!(
                runtime
                    .pending_respawn(points[warned].0)
                    .map(|pending| pending.step),
                Some(spawn::RespawnStep::Due)
            );
        }
        assert_eq!(runtime.spawn_schedule.len(), total);
    }

    /// The single-source path enforces `CREATUREAI0-RL-01` against the points already
    /// realized: up to the ceiling passes, one past it refuses with no admission.
    #[test]
    fn single_spawn_source_respects_the_channel_point_ceiling() {
        let (continuity, mut carrier) = carrier(4);
        let context = spawn_position_context(&continuity);
        let per_source = CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX;
        // Bookkeeping-only sources stand for already realized points; their cells name no
        // creature, so no slot is spent on them.
        let filler = |source: u16, points: usize| SpawnRealization {
            source: SpawnSourceId(source),
            definition: row_definition(0, points),
            cells: vec![
                SpawnCellState {
                    live: None,
                    attempt: 0,
                    successor: 0,
                };
                points
            ],
        };
        let full_sources = CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX / per_source;
        carrier.spawns = (0..full_sources - 1)
            .map(|source| filler(u16::try_from(source).expect("small"), per_source))
            .collect();
        carrier.spawns.push(filler(
            u16::try_from(full_sources - 1).expect("small"),
            per_source - 1,
        ));
        let next = SpawnSourceId(u16::try_from(full_sources).expect("small"));
        carrier
            .realize_spawn(&continuity, next, row_definition(0, 1), context)
            .expect("exactly CREATUREAI0-RL-01 points");
        let slots_before = carrier.slots.clone();
        let spawns_before = carrier.spawns.len();
        assert_eq!(
            carrier.realize_spawn(
                &continuity,
                SpawnSourceId(next.0 + 1),
                row_definition(1, 1),
                context
            ),
            Err(CarrierError::CapacityExceeded)
        );
        assert_eq!(carrier.slots, slots_before);
        assert_eq!(carrier.spawns.len(), spawns_before);
    }

    /// Sources realized out of order still land in ascending order, and every lookup finds
    /// them by binary search.
    #[test]
    fn spawn_sources_stay_ordered_for_lookup() {
        let (continuity, mut carrier) = carrier(8);
        let context = spawn_position_context(&continuity);
        for (source, x) in [(7_u16, 0), (2, 1), (40_000, 2), (3, 3)] {
            carrier
                .realize_spawn(
                    &continuity,
                    SpawnSourceId(source),
                    row_definition(x, 1),
                    context,
                )
                .expect("realizes");
        }
        let order = carrier
            .spawns
            .iter()
            .map(|spawn| spawn.source.0)
            .collect::<Vec<_>>();
        assert_eq!(order, [2, 3, 7, 40_000]);
        for source in [2_u16, 3, 7, 40_000] {
            assert!(carrier.spawn_cell_state(SpawnSourceId(source), 0).is_ok());
        }
        assert!(matches!(
            carrier.spawn_cell_state(SpawnSourceId(4), 0),
            Err(CarrierError::UnknownSpawnSource)
        ));
        assert_eq!(
            carrier.realize_spawn(&continuity, SpawnSourceId(7), row_definition(4, 1), context),
            Err(CarrierError::DuplicateSpawnSource)
        );
    }

    /// Removal no longer scans the spawn cells, so a cell can keep naming a removed creature.
    /// That stale reference never reads as the slot's later occupant, and the respawn's
    /// best-effort removal of it never removes that occupant.
    #[test]
    fn a_removed_spawned_creature_reads_as_gone_after_its_slot_is_reused() {
        let (continuity, mut carrier) = carrier(1);
        let context = spawn_position_context(&continuity);
        carrier
            .realize_spawn(&continuity, SpawnSourceId(0), row_definition(0, 1), context)
            .expect("realizes");
        let stale = carrier
            .spawn_cell_state(SpawnSourceId(0), 0)
            .expect("cell tracked")
            .live
            .expect("live rat");
        carrier.remove(&continuity, stale.0).expect("removed");
        let occupant = ExactActorRef(
            carrier
                .admit_creature(&continuity, ActorState(0), "oteryn:creature/rat", 20)
                .expect("the slot is reused"),
        );
        assert_eq!(occupant.0.actor_local_id, stale.0.actor_local_id);
        assert_eq!(
            carrier
                .spawn_cell_state(SpawnSourceId(0), 0)
                .expect("cell tracked")
                .live,
            Some(stale)
        );
        assert!(!carrier.creature_alive(&continuity, stale));
        assert!(carrier.creature_alive(&continuity, occupant));
        assert!(matches!(
            carrier.remove(&continuity, stale.0),
            Err(CarrierError::StaleActorGeneration)
        ));
        assert!(carrier.creature_alive(&continuity, occupant));
    }

    #[test]
    fn spawn_population_and_placement_cells_mismatch_rejected_both_directions() {
        // P2 (Codex review, PR #1193): §4.3 "one declared cell per creature" -- fewer cells
        // than the population must not clamp down, and extra unused cells must not be
        // tolerated either, since `realize_spawn`/`resolve_respawn_timer` would otherwise be
        // able to respawn more creatures than the declared population.
        assert_eq!(
            SpawnDefinition::new(
                vec![(0, 0, 0)],
                2,
                60_000_000,
                5_000_000,
                "oteryn:creature/rat".to_string(),
                20,
            ),
            Err(CarrierError::InvalidSpawnDefinition),
            "population above the declared cell count is rejected"
        );
        assert_eq!(
            SpawnDefinition::new(
                vec![(0, 0, 0), (1, 0, 0)],
                1,
                60_000_000,
                5_000_000,
                "oteryn:creature/rat".to_string(),
                20,
            ),
            Err(CarrierError::InvalidSpawnDefinition),
            "extra, unused cells beyond the population are rejected"
        );
        assert!(
            SpawnDefinition::new(
                vec![(0, 0, 0), (1, 0, 0)],
                2,
                60_000_000,
                5_000_000,
                "oteryn:creature/rat".to_string(),
                20,
            )
            .is_ok(),
            "an exact population/cell-count match is accepted"
        );
    }

    /// Kills the live creature at `cell_index` of `source` (a direct lethal commit through the
    /// exact-actor commit path, mirroring `CombatDeathFixture::strike`), so its cell's respawn
    /// can be exercised.
    fn kill_spawn_cell(
        continuity: &NamespaceContinuityGuard,
        carrier: &mut ChannelActorCarrier,
        source: SpawnSourceId,
        cell_index: usize,
    ) -> ExactActorRef {
        let dead = carrier
            .spawn_cell_state(source, cell_index)
            .expect("cell tracked")
            .live
            .expect("cell has a live rat to kill");
        carrier
            .current_owner_exact_commit(continuity)
            .commit_damage(
                dead,
                OwnerDamageCommand {
                    target: b"oteryn:creature/rat",
                    occurrence: b"kill",
                    binding: b"kill\0respawn-test.v1",
                    damage: 20,
                },
            )
            .expect("lethal strike");
        dead
    }

    /// VIS-3 (Codex 4178196302): the occupied index names exactly the occupied slots after
    /// admissions, removals, slot reuse, a spawn and a respawn, and holds no more than them.
    #[test]
    fn the_occupied_index_is_exactly_the_occupied_slots() {
        let by_scan = |carrier: &ChannelActorCarrier| -> Vec<u32> {
            (0_u32..)
                .zip(carrier.slots.iter())
                .filter(|(_, slot)| {
                    matches!(
                        slot,
                        Slot::Occupied { .. }
                            | Slot::CreatureOccupied { .. }
                            | Slot::CreatureReserved { .. }
                    )
                })
                .map(|(index, _)| index)
                .collect()
        };
        let indexed = |carrier: &ChannelActorCarrier| -> Vec<u32> {
            let mut occupied = carrier.occupied.clone();
            occupied.sort_unstable();
            occupied
        };
        let (continuity, mut carrier) = carrier(16);
        assert!(carrier.occupied.is_empty());
        assert!(carrier.occupied.capacity() >= 16);
        let first = carrier.admit(&continuity, ActorState(1)).expect("admit");
        let second = carrier.admit(&continuity, ActorState(2)).expect("admit");
        carrier.admit(&continuity, ActorState(3)).expect("admit");
        assert_eq!(indexed(&carrier), [0, 1, 2]);
        carrier.remove(&continuity, second).expect("remove");
        assert_eq!(indexed(&carrier), by_scan(&carrier));
        assert_eq!(indexed(&carrier), [0, 2]);
        // The freed slot is reused, and listed once.
        carrier.admit(&continuity, ActorState(4)).expect("admit");
        assert_eq!(indexed(&carrier), [0, 1, 2]);
        carrier.remove(&continuity, first).expect("remove");
        let context = spawn_position_context(&continuity);
        carrier
            .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
            .expect("D116 realizes");
        assert_eq!(indexed(&carrier), by_scan(&carrier));
        kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 0);
        carrier
            .resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context)
            .expect("resolves");
        assert_eq!(indexed(&carrier), by_scan(&carrier));
        assert!(carrier.occupied.len() < carrier.slots.len());
    }

    #[test]
    fn respawn_admits_a_fresh_generation_when_the_cell_is_free() {
        let (continuity, mut carrier) = carrier(4);
        let context = spawn_position_context(&continuity);
        carrier
            .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
            .expect("D116 realizes");
        let dead = kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 0);

        let resolution = carrier
            .resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context)
            .expect("resolves");
        assert!(
            matches!(resolution, RespawnResolution::Admitted(_)),
            "expected an admission when the cell is free: {resolution:?}"
        );
        let RespawnResolution::Admitted(new_actor) = resolution else {
            unreachable!("checked above");
        };
        assert_ne!(
            new_actor.0.actor_local_generation, dead.0.actor_local_generation,
            "D52: the dead actor's generation is never reused"
        );
        assert!(
            carrier
                .current_owner_exact_lookup(&continuity)
                .contains(new_actor)
        );
        assert_eq!(
            carrier
                .spawn_cell_state(SpawnSourceId(1), 0)
                .expect("cell tracked")
                .live,
            Some(new_actor)
        );
    }

    #[test]
    fn respawn_with_an_invalid_context_mutates_nothing_and_returns_err() {
        // P1 (Codex review, PR #1193): the context is validated before the dead actor is
        // removed or a replacement admitted, so a rejected respawn never leaves the cell
        // pointing at a removed actor while an unpositioned replacement stays admitted.
        let (continuity, mut carrier) = carrier(4);
        let context = spawn_position_context(&continuity);
        carrier
            .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
            .expect("D116 realizes");
        let dead = kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 0);

        let mut invalid_context = context;
        invalid_context.coordinate_frame_marker = 0;
        let slots_before = carrier.slots.clone();
        let free_head_before = carrier.free_head;
        let spawns_before = carrier.spawns.clone();
        assert_eq!(
            carrier.resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, invalid_context),
            Err(CarrierError::InvalidPreProductionPositionContext)
        );
        assert_eq!(carrier.slots, slots_before);
        assert_eq!(carrier.free_head, free_head_before);
        assert_eq!(carrier.spawns, spawns_before);
        assert_eq!(
            carrier
                .spawn_cell_state(SpawnSourceId(1), 0)
                .expect("cell tracked")
                .live,
            Some(dead),
            "the cell still names the original dead actor"
        );

        let resolution = carrier
            .resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context)
            .expect("a valid context still resolves afterward");
        assert!(matches!(resolution, RespawnResolution::Admitted(_)));
    }

    #[test]
    fn respawn_postpones_three_times_then_terminates_skipped_and_schedules_a_successor() {
        let (continuity, mut carrier) = carrier(6);
        let context = spawn_position_context(&continuity);
        carrier
            .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
            .expect("D116 realizes");
        kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 0);
        // An unrelated live creature occupies cell 0's exact position, blocking respawn.
        let blocker = carrier
            .admit_creature(&continuity, ActorState(9), "oteryn:creature/blocker", 20)
            .expect("blocker admits");
        carrier
            .initialize_position(
                &continuity,
                blocker,
                context,
                LocalPosition {
                    x: 1,
                    y: 0,
                    floor: 7,
                },
            )
            .expect("blocker occupies cell 0");

        for expected_attempt in 1..=AI01_SPAWN_OCCUPANCY_RETRIES_MAX {
            assert_eq!(
                carrier.resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context),
                Ok(RespawnResolution::Postponed {
                    next_attempt: expected_attempt
                })
            );
        }
        assert_eq!(
            carrier.resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context),
            Ok(RespawnResolution::Skipped { next_successor: 1 }),
            "the third retry still finds the cell occupied: terminal SKIPPED"
        );
        let state = carrier
            .spawn_cell_state(SpawnSourceId(1), 0)
            .expect("cell tracked");
        assert_eq!(
            state.attempt, 0,
            "a fresh successor chain starts at attempt 0"
        );
        assert_eq!(state.successor, 1);

        // The blocker leaves; the successor chain's own respawn now succeeds.
        carrier
            .remove(&continuity, blocker)
            .expect("blocker leaves");
        let resolution = carrier
            .resolve_respawn_timer(&continuity, SpawnSourceId(1), 0, context)
            .expect("resolves once the cell is free");
        assert!(matches!(resolution, RespawnResolution::Admitted(_)));
    }

    #[test]
    fn respawn_timer_lane_binding_items_and_deadline_state_never_collapses_distinct_cells() {
        // AI-1 binding items applied to respawn occurrences (carried over per the coordinator's
        // instruction): (a) bounded replay evidence via `SpawnCellState.attempt`/`.successor`,
        // which `resolve_respawn_timer` alone advances, so a `RespawnOccurrence` tuple can never
        // repeat; (b) `schedule` itself only accepts a fence-issued `RuntimeWorkStamp` (AI-1's
        // own enforcement, exercised here exactly as its own tests do).
        let (continuity, mut carrier) = carrier(6);
        let context = spawn_position_context(&continuity);
        carrier
            .realize_spawn(&continuity, SpawnSourceId(1), d116_definition(), context)
            .expect("D116 realizes");
        let dead_0 = kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 0);
        let dead_1 = kill_spawn_cell(&continuity, &mut carrier, SpawnSourceId(1), 1);

        use crate::foundation::owner_timer::{
            CatchUpPolicy, FamilyPolicy, OwnerTimerError, OwnerTimerLane, SemanticTimeMicros,
            VirtualOwnerClock,
        };
        use crate::foundation::{RuntimeScopeRefV1, ScopeRuntimeFence};

        let scope = RuntimeScopeRefV1::channel(continuity.world_id, continuity.channel_id);
        let generation = continuity.current_generation;
        let mut lane: OwnerTimerLane<RespawnFamily, RespawnOccurrence> =
            OwnerTimerLane::for_generation(
                scope,
                generation,
                [(
                    RespawnFamily::Respawn,
                    FamilyPolicy {
                        max_pending: AI01_PENDING_RESPAWN_TIMERS_PER_DEAD_ACTOR,
                        catch_up: CatchUpPolicy::DeadlineState,
                    },
                )],
            )
            .expect("cap within its registered maximum");
        let mut owner_fence = ScopeRuntimeFence::from_external_grant(generation).with_scope(scope);
        let due = SemanticTimeMicros::from_micros(60_000_000);

        for (dead, index) in [(dead_0, 0_u8), (dead_1, 1_u8)] {
            let ordinal = owner_fence
                .accept_input(generation)
                .expect("issue a live ordinal");
            let stamp = owner_fence.stamp(ordinal);
            lane.schedule(
                &owner_fence,
                stamp,
                RespawnFamily::Respawn,
                RespawnOccurrence {
                    source: SpawnSourceId(1),
                    cell_index: index,
                    dead_actor: dead,
                    successor: 0,
                    attempt: 1,
                },
                Some(dead),
                due,
            )
            .expect("schedule");
        }
        // Binding item (a): the exact same occurrence can never be scheduled twice.
        let ordinal = owner_fence
            .accept_input(generation)
            .expect("issue a live ordinal");
        let stamp = owner_fence.stamp(ordinal);
        assert_eq!(
            lane.schedule(
                &owner_fence,
                stamp,
                RespawnFamily::Respawn,
                RespawnOccurrence {
                    source: SpawnSourceId(1),
                    cell_index: 0,
                    dead_actor: dead_0,
                    successor: 0,
                    attempt: 1,
                },
                Some(dead_0),
                due,
            ),
            Err(OwnerTimerError::DuplicateOccurrence)
        );

        let clock = VirtualOwnerClock::new(due);
        let fired = lane.drain_due(&clock, &owner_fence, |_| true);
        // DeadlineState (distinct from AI-1's SkipToLatest): both distinct cells' occurrences
        // fire, never collapsed into one.
        assert_eq!(fired.len(), 2);
        let occurrences: std::collections::HashSet<_> = fired
            .iter()
            .map(|timer| timer.occurrence.cell_index)
            .collect();
        assert_eq!(occurrences, std::collections::HashSet::from([0, 1]));

        for fired_timer in &fired {
            let resolution = carrier
                .resolve_respawn_timer(
                    &continuity,
                    fired_timer.occurrence.source,
                    fired_timer.occurrence.cell_index as usize,
                    context,
                )
                .expect("resolves");
            assert!(matches!(resolution, RespawnResolution::Admitted(_)));
        }
    }

    #[test]
    fn carrier_loss_does_not_release_same_generation_claim() {
        let mut continuity = continuity(50, 9);
        let carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
            .expect("first bootstrap");
        drop(carrier);
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1),
            Err(CarrierError::NamespaceAlreadyClaimed)
        );
    }

    #[test]
    fn claimed_namespace_rejects_before_the_allocation_sentinel() {
        let mut continuity = continuity(55, 3);
        let carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
            .expect("first bootstrap claims namespace");
        drop(carrier);

        // This huge capacity is arithmetically valid and would hit the test
        // allocation-failure sentinel if authority preflight did not win.
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(
                &mut continuity,
                TEST_ALLOCATION_FAILURE_CAPACITY,
            ),
            Err(CarrierError::NamespaceAlreadyClaimed)
        );
    }

    #[test]
    fn allocation_failure_does_not_consume_namespace_claim() {
        let mut continuity = continuity(56, 3);
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(
                &mut continuity,
                TEST_ALLOCATION_FAILURE_CAPACITY,
            ),
            Err(CarrierError::AllocationFailed)
        );
        assert!(!continuity.current_generation_claimed);

        let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
            .expect("retry after allocation failure");
        assert!(continuity.current_generation_claimed);
        assert!(carrier.admit(&continuity, ActorState(8)).is_ok());
    }

    #[test]
    fn advancing_live_continuity_immediately_fences_all_old_operations() {
        let mut continuity = continuity(60, 4);
        let mut carrier =
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1).expect("bootstrap");
        let actor_ref = carrier.admit(&continuity, ActorState(7)).expect("admit");

        continuity.advance(grant(60, 5)).expect("strictly newer");
        let slots_before = carrier.slots.clone();

        assert_eq!(
            carrier.admit(&continuity, ActorState(9)),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(
            carrier.lookup(&continuity, actor_ref),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(
            carrier.remove(&continuity, actor_ref),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(carrier.slots, slots_before);
    }

    #[test]
    fn stale_equal_and_cross_scope_advances_leave_guard_unchanged() {
        let mut continuity = continuity(70, 8);
        let before = (
            continuity.world_id,
            continuity.channel_id,
            continuity.current_generation,
            continuity.current_generation_claimed,
        );

        assert_eq!(
            continuity.advance(grant(70, 8)),
            Err(CarrierError::ContinuityGenerationNotNewer)
        );
        assert_eq!(
            continuity.advance(grant(70, 7)),
            Err(CarrierError::ContinuityGenerationNotNewer)
        );
        assert_eq!(
            continuity.advance(grant(80, 9)),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(
            (
                continuity.world_id,
                continuity.channel_id,
                continuity.current_generation,
                continuity.current_generation_claimed,
            ),
            before
        );
    }

    #[test]
    fn strictly_newer_advance_permits_one_fresh_namespace_and_keeps_old_fenced() {
        let mut continuity = continuity(90, 2);
        let mut old_carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
            .expect("old bootstrap");
        let old_ref = old_carrier
            .admit(&continuity, ActorState(1))
            .expect("old admit");

        continuity.advance(grant(90, 3)).expect("advance");
        let mut new_carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
            .expect("fresh generation bootstrap");

        assert_eq!(new_carrier.scope_generation.get(), 3);
        let new_ref = new_carrier
            .admit(&continuity, ActorState(2))
            .expect("fresh carrier admits under live continuity");
        assert_eq!(new_carrier.lookup(&continuity, new_ref), Ok(&ActorState(2)));
        assert_eq!(
            old_carrier.lookup(&continuity, old_ref),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1),
            Err(CarrierError::NamespaceAlreadyClaimed)
        );
    }

    /// VIS-3: `visible_entities` reads committed positioned players and live creatures under the
    /// pinned context, and nothing else; it changes nothing.
    #[test]
    fn visible_entities_are_the_pinned_players_and_live_creatures() {
        let mut runtime = runtime(6);
        let at = |x: i32| MovementLocalPosition { x, y: 5, floor: 7 };
        let player = |runtime: &mut ChannelRuntimeV1, raw: u64, x: i32| {
            let reservation = runtime
                .reserve_fresh_session(session(raw))
                .expect("reserve");
            let actor = runtime.commit_fresh_session(reservation).expect("commit");
            runtime
                .initialize_movement_test_position(actor, at(x))
                .expect("position");
            actor
        };
        let shown = player(&mut runtime, 30, 1);
        // Positioned under another context: never shown.
        let other_context = player(&mut runtime, 31, 2);
        // Reserved but not committed: never shown.
        runtime.reserve_fresh_session(session(32)).expect("reserve");
        let live = runtime.admit_test_creature(at(3)).expect("creature");
        // Dead: never shown.
        runtime.admit_test_creature(at(4)).expect("creature");
        let pinned = runtime.pinned_position_context();
        for slot in runtime.carrier.slots.iter_mut() {
            match slot {
                Slot::Occupied {
                    position: Some(version),
                    ..
                }
                | Slot::CreatureOccupied {
                    position: Some(version),
                    ..
                } if version.position.x != 2 => version.context = pinned,
                _ => {}
            }
            if let Slot::CreatureOccupied {
                health,
                position: Some(version),
                ..
            } = slot
                && version.position.x == 4
            {
                *health = 0;
            }
        }
        let before = runtime.carrier.slots.clone();

        let visible = runtime.visible_entities();
        assert_eq!(
            visible
                .players
                .iter()
                .map(|entry| entry.actor)
                .collect::<Vec<_>>(),
            [shown]
        );
        assert_ne!(shown, other_context);
        assert_eq!(visible.players[0].position, at(1));
        assert_eq!(
            visible.players[0].generation,
            shown.0.actor_local_generation.0
        );
        assert_eq!(
            visible
                .creatures
                .iter()
                .map(|entry| entry.actor)
                .collect::<Vec<_>>(),
            [live]
        );
        assert_eq!(visible.creatures[0].position, at(3));
        // Read-only.
        assert_eq!(runtime.carrier.slots, before);
        assert_eq!(runtime.visible_entities(), visible);
    }
}

// Main reconciliation packet Foundation-1: source heal only, no lifecycle takeover.
// Append inside physical Foundation carrier. Loaded typed membership qualification is separate.
#[derive(Debug, Clone)]
pub struct CreatureSelfHealRegistration {
    world: WorldId,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    target: Box<str>,
    ability: Box<str>,
    max_health: i64,
    minimum: i64,
    maximum: i64,
}
/// SPAWN-CEILINGS-1 (Codex 4243039220): replay memos keyed by an exact creature and one of its
/// entries (a summon entry or a defence Ability), deterministic ordered maps throughout. The
/// memos of a creature live under its slot, which holds one creature at a time: a creature's
/// first touch drops what the slot's former occupant left (at most its entries), so a lookup,
/// an insert or that cleanup costs the logarithm of the table plus the entries of one slot,
/// never a pass over every memo. The memos of a dead creature whose slot nobody reuses stay
/// until [`Self::prune`], which the owners run only when the table is full.
#[derive(Debug)]
pub struct ActorMemoTable<K, V> {
    slots: BTreeMap<u32, ActorMemos<K, V>>,
    len: usize,
}
#[derive(Debug)]
struct ActorMemos<K, V> {
    actor: ExactActorRef,
    entries: BTreeMap<K, V>,
}
impl<K, V> Default for ActorMemoTable<K, V> {
    fn default() -> Self {
        Self {
            slots: BTreeMap::new(),
            len: 0,
        }
    }
}
impl<K: Ord, V> ActorMemoTable<K, V> {
    /// Every memo held, a dead creature's included.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Whether the table holds no memos.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// The memos of exactly `actor`.
    pub fn actor_len(&self, actor: ExactActorRef) -> usize {
        self.slots
            .get(&actor.actor_local_id())
            .filter(|memos| memos.actor == actor)
            .map_or(0, |memos| memos.entries.len())
    }
    pub fn get<Q>(&self, actor: ExactActorRef, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord + ?Sized,
    {
        if self.is_empty() {
            return None;
        }
        self.slots
            .get(&actor.actor_local_id())
            .filter(|memos| memos.actor == actor)?
            .entries
            .get(key)
    }
    /// Drops the memos `actor`'s slot holds for any other actor: its former occupant, dead
    /// because the slot was taken again, or one of another scope. Returns how many it dropped.
    pub fn evict_stale_at(&mut self, actor: ExactActorRef) -> usize {
        let slot = actor.actor_local_id();
        if self
            .slots
            .get(&slot)
            .is_none_or(|memos| memos.actor == actor)
        {
            return 0;
        }
        let dropped = self
            .slots
            .remove(&slot)
            .map_or(0, |memos| memos.entries.len());
        self.len -= dropped;
        dropped
    }
    /// Sets `actor`'s memo of `key`, first dropping its slot's stale memos; returns the memo it
    /// replaced.
    pub fn insert(&mut self, actor: ExactActorRef, key: K, value: V) -> Option<V> {
        self.evict_stale_at(actor);
        let memos = self
            .slots
            .entry(actor.actor_local_id())
            .or_insert_with(|| ActorMemos {
                actor,
                entries: BTreeMap::new(),
            });
        let replaced = memos.entries.insert(key, value);
        if replaced.is_none() {
            self.len += 1;
        }
        replaced
    }
    /// Drops the memos of every actor `live` refuses: one pass over the slots that hold memos,
    /// which the owners run only when the table is full.
    pub fn prune(&mut self, live: impl Fn(ExactActorRef) -> bool) {
        let mut len = 0;
        self.slots.retain(|_, memos| {
            let keep = live(memos.actor);
            if keep {
                len += memos.entries.len();
            }
            keep
        });
        self.len = len;
    }
}
#[derive(Debug, Default)]
pub struct CreatureSelfHealLedger {
    /// By creature and defence Ability (Codex 4243039220).
    entries: ActorMemoTable<Box<str>, CreatureSelfHealEntry>,
}
#[derive(Debug)]
struct CreatureSelfHealEntry {
    sequence: u64,
    draw: i64,
    result: OwnerDamageResult,
}
impl ChannelRuntimeV1 {
    /// Composition loader supplies an exact current Creature->defense Ability->Heal Effect->Range
    /// chain. Native registration fences the immutable definition facts to this activation.
    /// Never call this from network input or infer membership merely from an artifact digest.
    pub fn bind_creature_self_heal(
        &self,
        target: &str,
        ability: &str,
        max_health: u64,
        minimum: u64,
        maximum: u64,
    ) -> Result<CreatureSelfHealRegistration, CarrierError> {
        if !target.starts_with("oteryn:creature.")
            || !ability.starts_with("oteryn:ability.")
            || max_health == 0
            || minimum > maximum
            || maximum > i64::MAX as u64
            || max_health > i64::MAX as u64
        {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        Ok(CreatureSelfHealRegistration {
            world: self.content.world_id,
            activation: self.content.activation_sequence,
            server: self.content.server_artifact_digest,
            client: self.content.client_artifact_digest,
            frame: self.content.frame_binding_digest,
            map: self.content.map_revision_digest,
            target: target.into(),
            ability: ability.into(),
            max_health: max_health as i64,
            minimum: minimum as i64,
            maximum: maximum as i64,
        })
    }
    /// Actual native creature HP writer, no Effect fixture map. Ordered think/cast sequence per (caster, defense ability). Independent defenses may
    /// execute in the same Think, including Think0; A+B+retryA cannot reapply A.
    pub fn commit_creature_self_heal(
        &mut self,
        ledger: &mut CreatureSelfHealLedger,
        registration: &CreatureSelfHealRegistration,
        actor: ExactActorRef,
        sequence: u64,
        server_draw: i64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        if registration.world != self.content.world_id
            || registration.activation != self.content.activation_sequence
            || registration.server != self.content.server_artifact_digest
            || registration.client != self.content.client_artifact_digest
            || registration.frame != self.content.frame_binding_digest
            || registration.map != self.content.map_revision_digest
        {
            return Err(CarrierError::PositionContextMismatch);
        }
        if !(registration.minimum..=registration.maximum).contains(&server_draw) {
            return Err(CarrierError::InvalidDamage);
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        self.carrier.assert_slot_spell_unreserved(index)?;
        let before = match &self.carrier.slots[index] {
            Slot::CreatureOccupied { generation, .. }
                if *generation != actor.0.actor_local_generation.0 =>
            {
                return Err(CarrierError::StaleActorGeneration);
            }
            Slot::CreatureOccupied {
                target_identity,
                health,
                ..
            } if target_identity.as_ref() == registration.target.as_bytes() && *health > 0 => {
                *health
            }
            Slot::CreatureOccupied {
                target_identity, ..
            } if target_identity.as_ref() != registration.target.as_bytes() => {
                return Err(CarrierError::CreatureTargetMismatch);
            }
            _ => return Err(CarrierError::CreatureNotActionable),
        };
        if before > registration.max_health {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        // The live actor's slot drops its former occupant's entries; the other dead actors'
        // entries go only when the ledger is full (Codex 4243039220).
        ledger.entries.evict_stale_at(actor);
        let known = match ledger.entries.get(actor, &*registration.ability) {
            Some(entry) => {
                if sequence < entry.sequence {
                    return Err(CarrierError::StaleAttackerSequence);
                }
                if sequence == entry.sequence {
                    if entry.draw != server_draw {
                        return Err(CarrierError::PlanConflict);
                    }
                    return Ok(OwnerDamageResult {
                        applied: false,
                        ..entry.result
                    });
                }
                true
            }
            None => false,
        };
        if !known {
            if ledger.entries.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX {
                ledger.entries.prune(|e| self.contains_live_creature(e));
            }
            if ledger.entries.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX
                || ledger.entries.actor_len(actor) >= CREATUREAI0_DEFENCES_PER_CREATURE_MAX
            {
                return Err(CarrierError::AllocationFailed);
            }
        }
        // Existing native authoring admits at most8 defenses; AI active actors ceiling64.
        // Aggregate current-owner ScopeRuntimeFence is required at the composition call.
        // Allocate source binding before native HP write, including replay state replacement.
        let ability = registration.ability.clone();
        let after = before
            .saturating_add(server_draw)
            .min(registration.max_health);
        let Slot::CreatureOccupied { health, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        *health = after;
        let result = OwnerDamageResult {
            applied: true,
            health_before: before,
            health_after: after,
        };
        let entry = CreatureSelfHealEntry {
            sequence,
            draw: server_draw,
            result,
        };
        ledger.entries.insert(actor, ability, entry);
        Ok(result)
    }
}

// Actual HP component fixture only. Loaded source-graph proof is separate.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
mod creature_self_heal_owner_tests {
    use super::*;
    #[test]
    fn independent_defenses_think0_a_b_retry_a_do_not_redamage_or_reheal() {
        fn id(tag: u8) -> [u8; 16] {
            let mut b = [0; 16];
            b[6] = 0x70;
            b[8] = 0x80;
            b[15] = tag;
            b
        }
        let world_id =
            WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
        let mut r = ChannelRuntimeV1::from_committed_assignment(
            world_id,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::from_activation(
                world_id,
                1,
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                (100, 100, 7),
            ),
        )
        .expect("valid native fixture operation must succeed");
        let registration = r
            .bind_creature_self_heal(
                "oteryn:creature.boreth",
                "oteryn:ability.creature.boreth.defense-1",
                1400,
                100,
                200,
            )
            .expect("valid native fixture operation must succeed");
        assert_eq!(
            (
                registration.max_health,
                registration.minimum,
                registration.maximum
            ),
            (1400, 100, 200)
        );
        let a = r
            .carrier
            .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.boreth", 1000)
            .expect("valid native fixture operation must succeed");
        let actor = ExactActorRef(a);
        let mut ledger = CreatureSelfHealLedger::default();
        assert!(
            r.commit_creature_self_heal(&mut ledger, &registration, actor, 1, 99)
                .is_err()
        );
        let first = r
            .commit_creature_self_heal(&mut ledger, &registration, actor, 0, 200)
            .expect("valid native fixture operation must succeed");
        assert_eq!((first.health_before, first.health_after), (1000, 1200)); // Independent defense B is an explicit owner-component fixture, not a donor catalog claim.
        let b = r
            .bind_creature_self_heal(
                "oteryn:creature.boreth",
                "oteryn:ability.creature.boreth.fixture_second_heal",
                1400,
                100,
                100,
            )
            .expect("valid native fixture operation must succeed");
        let interleaved = r
            .commit_creature_self_heal(&mut ledger, &b, actor, 0, 100)
            .expect("valid native fixture operation must succeed");
        assert_eq!(interleaved.health_after, 1300);
        assert!(
            !r.commit_creature_self_heal(&mut ledger, &registration, actor, 0, 200)
                .expect("valid native fixture operation must succeed")
                .applied
        );
        let index = r
            .carrier
            .validate_ref(&r.continuity, actor.0)
            .expect("valid native fixture operation must succeed");
        assert!(matches!(
            r.carrier.slots[index],
            Slot::CreatureOccupied { health: 1300, .. }
        ));
        let second = r
            .commit_creature_self_heal(&mut ledger, &registration, actor, 1, 200)
            .expect("valid native fixture operation must succeed");
        assert_eq!(second.health_after, 1400);
        let capped = r
            .commit_creature_self_heal(&mut ledger, &registration, actor, 2, 200)
            .expect("valid native fixture operation must succeed");
        assert_eq!(capped.health_after, 1400);
        assert!(
            !r.commit_creature_self_heal(&mut ledger, &registration, actor, 2, 200)
                .expect("valid native fixture operation must succeed")
                .applied
        );
        assert_eq!(
            r.commit_creature_self_heal(&mut ledger, &registration, actor, 0, 200)
                .expect_err("fixture must reject this invalid operation"),
            CarrierError::StaleAttackerSequence
        );
        assert_eq!(
            r.commit_creature_self_heal(&mut ledger, &registration, actor, 2, 100)
                .expect_err("fixture must reject this invalid operation"),
            CarrierError::PlanConflict
        );
        let mut bad = registration.clone();
        bad.server[0] ^= 1;
        assert!(
            r.commit_creature_self_heal(&mut ledger, &bad, actor, 4, 100)
                .is_err()
        );
        r.carrier
            .current_owner_exact_commit(&r.continuity)
            .commit_damage(
                actor,
                OwnerDamageCommand {
                    target: b"oteryn:creature.boreth",
                    occurrence: b"dead",
                    binding: b"dead\0health1400",
                    damage: 1400,
                },
            )
            .expect("valid native fixture operation must succeed");
        assert!(
            r.commit_creature_self_heal(&mut ledger, &registration, actor, 4, 100)
                .is_err()
        );
    }
}

impl ChannelRuntimeV1 {
    pub fn current_live_creature_identity(
        &self,
        actor: ExactActorRef,
    ) -> Result<&[u8], CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                generation,
                target_identity,
                health,
                ..
            } if *generation == actor.0.actor_local_generation.0 && *health > 0 => {
                Ok(target_identity.as_ref())
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }
}

impl ChannelRuntimeV1 {
    /// Source-qualified bounded draws from this physical Channel owner; exact current pin,
    /// all actor generations/identity/HP and output storage preflight before publication.
    /// Existing CreatureCommitState including its canonical conditions is preserved intact.
    pub fn commit_source_creature_heal_batch(
        &mut self,
        content: [u8; 32],
        requests: &[(ExactActorRef, String, u64, u64)],
    ) -> Result<Vec<OwnerDamageResult>, CarrierError> {
        if content != self.content.server_artifact_digest {
            return Err(CarrierError::PositionContextMismatch);
        }
        if requests.len() > 64 {
            return Err(CarrierError::AllocationFailed);
        }
        let original_len = requests.len();
        let (expanded, original_map) = self.expand_bone_shared_heals(requests)?;
        let requests = expanded.as_slice();
        let mut original_results = Vec::new();
        original_results
            .try_reserve(original_len)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut prepared = Vec::new();
        prepared
            .try_reserve(requests.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut results = Vec::new();
        results
            .try_reserve(requests.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (actor, key, max, draw) in requests {
            if *max == 0 || *max > i64::MAX as u64 || *draw > i64::MAX as u64 {
                return Err(CarrierError::InvalidCreatureHealth);
            }
            let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
            self.carrier.assert_slot_spell_unreserved(index)?;
            if prepared.iter().any(|(old, _)| *old == index) {
                return Err(CarrierError::PlanConflict);
            }
            let mut next = self.carrier.slots[index].clone();
            let Slot::CreatureOccupied {
                generation,
                target_identity,
                health,
                ..
            } = &mut next
            else {
                return Err(CarrierError::NotCreature);
            };
            if *generation != actor.0.actor_local_generation.0 {
                return Err(CarrierError::StaleActorGeneration);
            }
            if target_identity.as_ref() != key.as_bytes() || *health <= 0 {
                return Err(CarrierError::CreatureTargetMismatch);
            }
            if *health > *max as i64 {
                return Err(CarrierError::InvalidCreatureHealth);
            }
            let before = *health;
            let after = before.saturating_add(*draw as i64).min(*max as i64);
            *health = after;
            prepared.push((index, next));
            results.push(OwnerDamageResult {
                applied: true,
                health_before: before,
                health_after: after,
            });
        }
        for index in original_map {
            original_results.push(*results.get(index).ok_or(CarrierError::PlanConflict)?);
        }
        // All fallible work, including original result projection and canonical condition clones, precedes HP publication.
        for (index, next) in prepared {
            self.carrier.slots[index] = next;
        }
        Ok(original_results)
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
mod area_heal_batch_native_tests {
    use super::*;
    fn fixture() -> ChannelRuntimeV1 {
        let id = |t: u8| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
        let w = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
        ChannelRuntimeV1::from_committed_assignment(
            w,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::test(w),
        )
        .expect("valid native fixture operation must succeed")
    }
    #[test]
    fn all_target_preflight_rejects_second_before_first_hp_and_commits_caps() {
        let mut r = fixture();
        let a = ExactActorRef(
            r.carrier
                .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.rat", 5)
                .expect("valid native fixture operation must succeed"),
        );
        let b = ExactActorRef(
            r.carrier
                .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.rat", 10)
                .expect("valid native fixture operation must succeed"),
        );
        let mut q = vec![
            (a, "oteryn:creature.rat".to_string(), 20, 10),
            (b, "oteryn:creature.demon".to_string(), 20, 10),
        ];
        assert!(r.commit_source_creature_heal_batch([1; 32], &q).is_err());
        let ai = r
            .carrier
            .validate_ref(&r.continuity, a.0)
            .expect("valid native fixture operation must succeed");
        assert!(matches!(
            r.carrier.slots[ai],
            Slot::CreatureOccupied { health: 5, .. }
        ));
        q[1].1 = "oteryn:creature.rat".into();
        q[1].3 = 100;
        let receipts = r
            .commit_source_creature_heal_batch([1; 32], &q)
            .expect("valid native fixture operation must succeed");
        assert_eq!(
            (receipts[0].health_after, receipts[1].health_after),
            (15, 20)
        );
        let after = receipts[0].health_after;
        assert!(r.commit_source_creature_heal_batch([2; 32], &q).is_err());
        assert!(matches!(r.carrier.slots[ai],Slot::CreatureOccupied{health,..}if health==after));
    }
    #[test]
    fn stale_target_generation_and_duplicates_refuse_whole_batch() {
        let mut r = fixture();
        let a = ExactActorRef(
            r.carrier
                .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.rat", 5)
                .expect("valid native fixture operation must succeed"),
        );
        let q = (a, "oteryn:creature.rat".to_string(), 20, 10);
        assert_eq!(
            r.commit_source_creature_heal_batch([1; 32], &[q.clone(), q.clone()])
                .expect_err("fixture must reject this invalid operation"),
            CarrierError::PlanConflict
        );
        let i = r
            .carrier
            .validate_ref(&r.continuity, a.0)
            .expect("valid native fixture operation must succeed");
        if let Slot::CreatureOccupied { generation, .. } = &mut r.carrier.slots[i] {
            *generation += 1
        } else {
            assert!(
                matches!(r.carrier.slots[i], Slot::CreatureOccupied { .. }),
                "fixture must contain a native creature"
            )
        };
        assert_eq!(
            r.commit_source_creature_heal_batch([1; 32], &[q])
                .expect_err("fixture must reject this invalid operation"),
            CarrierError::StaleActorGeneration
        );
        assert!(matches!(
            r.carrier.slots[i],
            Slot::CreatureOccupied { health: 5, .. }
        ));
    }
}

impl ChannelRuntimeV1 {
    /// Same native physical HP owner used by the subsequent source-qualified heal batch.
    pub fn read_source_creature_health(
        &self,
        actor: ExactActorRef,
        key: &str,
        max: u64,
    ) -> Result<u64, CarrierError> {
        if max == 0 || max > i64::MAX as u64 {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied { generation, .. }
                if *generation != actor.0.actor_local_generation.0 =>
            {
                Err(CarrierError::StaleActorGeneration)
            }
            Slot::CreatureOccupied {
                target_identity,
                health,
                ..
            } if target_identity.as_ref() == key.as_bytes()
                && *health > 0
                && *health <= max as i64 =>
            {
                Ok(*health as u64)
            }
            _ => Err(CarrierError::CreatureTargetMismatch),
        }
    }
}
// Main reconciliation Foundation-2: intrinsic native summon lifecycle.
impl ChannelRuntimeV1 {
    pub fn matches_live_creature_identity(&self, actor: ExactActorRef, target: &[u8]) -> bool {
        self.current_live_creature_identity(actor)
            .is_ok_and(|identity| identity == target)
    }
}
// MONSTER-SUMMON-1 intrinsic ephemeral relation, owned by this physical Channel carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeSummonLink {
    parent: ExactActorRef,
    child: ExactActorRef,
    target: Box<[u8]>,
    origin: NativeSummonOrigin,
    death: Option<NativeSummonDeath>,
}
/// A slot and its generation: the part of an [`ExactActorRef`] that tells this carrier's actors
/// apart. A lookup by it still compares the whole reference.
type NativeSummonActorKey = (u32, u64);
fn native_summon_actor_key(actor: ExactActorRef) -> NativeSummonActorKey {
    (actor.0.actor_local_id.0, actor.0.actor_local_generation.0)
}
/// SPAWN-CEILINGS-1 (Codex 4243039217): the native summon links, indexed so that finding,
/// replacing or dropping a child's link and listing a parent's children cost the logarithm of the
/// links, never a pass over all of them. Links keep their admission order (the order of the
/// former list), so every per-parent and retirement pass visits them in the same order as before.
///
/// Equality compares the links in admission order, like the former list: not the ordinals nor
/// `retire_cursor`, which only order the links and schedule the bounded retirement work.
#[derive(Debug, Clone, Default)]
struct NativeSummonLinks {
    /// Every link by its admission ordinal.
    links: BTreeMap<u64, NativeSummonLink>,
    /// The admission ordinal of each child's link.
    by_child: BTreeMap<NativeSummonActorKey, u64>,
    /// The parent and admission ordinal of every link.
    by_parent: BTreeSet<(NativeSummonActorKey, u64)>,
    /// The admission ordinal of the next link.
    next: u64,
    /// The admission ordinal the next bounded retirement pass starts at
    /// (`NATIVE_SUMMON_RETIREMENT_LINKS_PER_CALL_MAX`).
    retire_cursor: u64,
}
impl PartialEq for NativeSummonLinks {
    fn eq(&self, other: &Self) -> bool {
        // The links in admission order, as the former list compared; `by_child` and `by_parent`
        // are derived from `links`, and the ordinals only order them.
        self.links.values().eq(other.links.values())
    }
}
impl Eq for NativeSummonLinks {}
impl NativeSummonLinks {
    fn len(&self) -> usize {
        self.links.len()
    }
    #[cfg(any(test, feature = "test-support"))]
    fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
    fn ordinal_of_child(&self, child: ExactActorRef) -> Option<u64> {
        let ordinal = *self.by_child.get(&native_summon_actor_key(child))?;
        self.links
            .get(&ordinal)
            .is_some_and(|link| link.child == child)
            .then_some(ordinal)
    }
    /// The link whose child is exactly `child`.
    fn child(&self, child: ExactActorRef) -> Option<&NativeSummonLink> {
        self.links.get(&self.ordinal_of_child(child)?)
    }
    fn child_mut(&mut self, child: ExactActorRef) -> Option<&mut NativeSummonLink> {
        let ordinal = self.ordinal_of_child(child)?;
        self.links.get_mut(&ordinal)
    }
    /// The links whose parent is exactly `parent`, in admission order.
    fn children(&self, parent: ExactActorRef) -> impl Iterator<Item = &NativeSummonLink> + '_ {
        let key = native_summon_actor_key(parent);
        self.by_parent
            .range((key, 0)..=(key, u64::MAX))
            .filter_map(|(_, ordinal)| self.links.get(ordinal))
            .filter(move |link| link.parent == parent)
    }
    fn has_children(&self, parent: ExactActorRef) -> bool {
        self.children(parent).next().is_some()
    }
    /// The links whose parent or child is `actor`, in admission order.
    fn touching(&self, actor: ActorRef) -> Vec<&NativeSummonLink> {
        let exact = ExactActorRef(actor);
        let mut found = self
            .ordinal_of_child(exact)
            .into_iter()
            .chain(
                self.by_parent
                    .range(
                        (native_summon_actor_key(exact), 0)
                            ..=(native_summon_actor_key(exact), u64::MAX),
                    )
                    .map(|(_, ordinal)| *ordinal),
            )
            .collect::<Vec<_>>();
        found.sort_unstable();
        found.dedup();
        found
            .into_iter()
            .filter_map(|ordinal| self.links.get(&ordinal))
            .filter(|link| link.parent.0 == actor || link.child.0 == actor)
            .collect()
    }
    fn insert(&mut self, link: NativeSummonLink) -> Result<(), CarrierError> {
        let child = native_summon_actor_key(link.child);
        if self.by_child.contains_key(&child) {
            return Err(CarrierError::PlanConflict);
        }
        let ordinal = self.next;
        self.next = ordinal
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        self.by_child.insert(child, ordinal);
        self.by_parent
            .insert((native_summon_actor_key(link.parent), ordinal));
        self.links.insert(ordinal, link);
        Ok(())
    }
    /// Drops the link whose child is exactly `child`.
    fn remove_child(&mut self, child: ExactActorRef) -> Option<NativeSummonLink> {
        let ordinal = self.ordinal_of_child(child)?;
        let link = self.links.remove(&ordinal)?;
        self.by_child.remove(&native_summon_actor_key(child));
        self.by_parent
            .remove(&(native_summon_actor_key(link.parent), ordinal));
        Some(link)
    }
    /// The children of at most `budget` links from the retirement cursor on, wrapping once, and
    /// the cursor after them.
    fn retirement_window(&self, budget: usize) -> Result<(Vec<ExactActorRef>, u64), CarrierError> {
        let mut window = Vec::new();
        window
            .try_reserve(budget.min(self.links.len()))
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut cursor = self.retire_cursor;
        for (ordinal, link) in self
            .links
            .range(self.retire_cursor..)
            .chain(self.links.range(..self.retire_cursor))
            .take(budget)
        {
            window.push(link.child);
            cursor = ordinal.saturating_add(1);
        }
        Ok((window, cursor))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSummonDeath {
    pub child: ExactActorRef,
    pub death: CreatureDeathOccurrenceKey,
    pub position: MovementLocalPosition,
}
impl ChannelRuntimeV1 {
    pub fn native_summon_role(
        &self,
        actor: ExactActorRef,
    ) -> Result<Option<ExactActorRef>, CarrierError> {
        self.carrier
            .validate_summon_actor(&self.continuity, actor)?;
        Ok(self.carrier.native_summons.child(actor).map(|l| l.parent))
    }
    pub fn native_summon_master(&self, actor: ExactActorRef) -> Option<ExactActorRef> {
        self.native_summon_role(actor).ok().flatten()
    }
    pub fn native_summon_count(&self, parent: ExactActorRef, target: Option<&str>) -> u32 {
        self.carrier
            .native_summons
            .children(parent)
            .filter(|l| {
                self.contains_live_creature(l.child)
                    && target.is_none_or(|t| l.target.as_ref() == t.as_bytes())
            })
            .count() as u32
    }
    pub fn native_summon_cell_free(&self, at: MovementLocalPosition) -> bool {
        !self.carrier.cell_occupied(
            self.pinned_position_context(),
            LocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        )
    }
    pub fn admit_native_summon(
        &mut self,
        current: &super::ScopeRuntimeFence,
        stamp: super::RuntimeWorkStamp,
        parent: ExactActorRef,
        spec: &NativeSummonAdmissionSpec,
        at: MovementLocalPosition,
    ) -> Result<ExactActorRef, CarrierError> {
        let b = self.binding();
        if !current.is_current_for_scope(
            super::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !current.accepts_stamp(stamp)
        {
            return Err(CarrierError::WrongScope);
        }
        self.retire_native_summons(current, stamp)?;
        if spec.content() != self.content.server_artifact_digest
            || !self.matches_live_creature_identity(parent, spec.parent_key().as_bytes())
            || self.native_summon_master(parent).is_some()
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        // The bounded pass above may not have reached this parent's links (Codex 4243039217):
        // retire its own dead children, at most 16, so its links stay its live children and so
        // at most 16 with the one admitted below.
        let mut dead = [None; 16];
        let mut count = 0;
        for link in self.carrier.native_summons.children(parent) {
            if count == 16 {
                return Err(CarrierError::CapacityExceeded);
            }
            let index = self
                .carrier
                .validate_summon_actor(&self.continuity, link.child)?;
            if matches!(
                &self.carrier.slots[index],
                Slot::CreatureOccupied { health: 0, .. }
            ) {
                dead[count] = Some(link.child);
                count += 1;
            }
        }
        if count > 0 {
            self.carrier
                .retire_dead_summons(&self.continuity, dead.iter().flatten().copied())?;
        }
        if self.native_summon_count(parent, None) >= spec.total()
            || self.native_summon_count(parent, Some(spec.child_key())) >= spec.count()
        {
            return Err(CarrierError::CapacityExceeded);
        }
        // `CREATUREAI0-RL-04` (Codex 4242954089), the bound `admit_creature` enforces, checked
        // before anything is written; it replaces the D57 envelope of 64 live creatures. Every
        // link names a distinct child that holds a slot until its removal drops the link, so
        // the links are bounded by the same value.
        if self.carrier.occupied.len() >= CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
            || self.carrier.native_summons.len() >= CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
            || !self.native_summon_cell_free(at)
        {
            return Err(CarrierError::CapacityExceeded);
        }
        let target = copy_bounded_binding(spec.child_key().as_bytes())?;
        let origin = spec.origin.try_owned()?;
        let context = self.pinned_position_context();
        self.carrier.validate_position_context(context)?;
        let slots = self.carrier.slots.clone();
        let free = self.carrier.free_head;
        let occupied = self.carrier.occupied.clone();
        let actor = self.carrier.admit_creature(
            &self.continuity,
            ActorState(0),
            spec.child_key(),
            spec.health(),
        )?;
        if let Err(e) = self.carrier.initialize_position(
            &self.continuity,
            actor,
            context,
            LocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        ) {
            self.carrier.slots = slots;
            self.carrier.free_head = free;
            self.carrier.occupied = occupied;
            return Err(e);
        }
        let child = ExactActorRef(actor);
        if let Err(e) = self.carrier.native_summons.insert(NativeSummonLink {
            parent,
            child,
            target,
            origin,
            death: None,
        }) {
            self.carrier.slots = slots;
            self.carrier.free_head = free;
            self.carrier.occupied = occupied;
            return Err(e);
        }
        Ok(child)
    }
    /// Summon death has its own sealed native key/position, never an ordinary corpse/reward.
    pub fn project_native_summon_death(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<NativeSummonDeath, CarrierError> {
        self.carrier
            .project_native_summon_death_inner(&self.continuity, actor)
    }
}
impl ChannelActorCarrier {
    fn validate_summon_actor(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor: ExactActorRef,
    ) -> Result<usize, CarrierError> {
        let index = self.validate_ref(continuity, actor.0)?;
        match &self.slots[index] {
            Slot::CreatureOccupied { generation, .. }
                if *generation == actor.0.actor_local_generation.0 =>
            {
                Ok(index)
            }
            Slot::Occupied { generation, .. }
                if *generation == actor.0.actor_local_generation.0 =>
            {
                Err(CarrierError::NotCreature)
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }

    /// Seals the death of each dead child of `dead` and removes it, as
    /// [`Self::project_native_summon_death_inner`] then [`Self::remove`] each, but drops the
    /// vacated slots from the occupied index and the attacker leases in one pass each (Codex
    /// 4243039217). On a failure the children removed so far stay removed, as before.
    fn retire_dead_summons(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        dead: impl IntoIterator<Item = ExactActorRef>,
    ) -> Result<(), CarrierError> {
        let dead = dead.into_iter();
        let mut vacated = Vec::new();
        vacated
            .try_reserve(dead.size_hint().0)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut result = Ok(());
        for child in dead {
            let removed = self
                .project_native_summon_death_inner(continuity, child)
                .and_then(|_| self.remove_keeping_indexes(continuity, child.0));
            match removed {
                Ok((_, index)) => {
                    if vacated.try_reserve(1).is_err() {
                        self.unindex_slot(index);
                    } else {
                        vacated.push(index);
                    }
                }
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        vacated.sort_unstable();
        if !vacated.is_empty() {
            self.occupied.retain(|index| {
                !usize::try_from(*index).is_ok_and(|index| vacated.binary_search(&index).is_ok())
            });
            self.attackers
                .retain(|entry| vacated.binary_search(&entry.index).is_err());
        }
        result
    }

    fn is_native_summon(&self, actor: ExactActorRef) -> bool {
        self.native_summons.child(actor).is_some()
            || self
                .native_summon_deaths
                .iter()
                .flatten()
                .any(|d| d.child == actor)
    }
    fn project_native_summon_death_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ExactActorRef,
    ) -> Result<NativeSummonDeath, CarrierError> {
        self.validate_ref(continuity, actor.0)?;
        if let Some(death) = self
            .native_summon_deaths
            .iter()
            .flatten()
            .find(|d| d.child == actor)
        {
            return Ok(*death);
        }
        self.validate_summon_actor(continuity, actor)?;
        if let Some(death) = self
            .native_summons
            .child(actor)
            .ok_or(CarrierError::NotCreature)?
            .death
        {
            return Ok(death);
        }
        let receipt = self.committed_lethal_receipt_inner(continuity, actor.0)?;
        self.validate_lethal_receipt(continuity, &receipt)?;
        let p = receipt.projection.position.position;
        let death = NativeSummonDeath {
            child: actor,
            death: receipt.projection.occurrence.death_key(),
            position: MovementLocalPosition {
                x: p.x,
                y: p.y,
                floor: p.floor,
            },
        };
        self.native_summons
            .child_mut(actor)
            .ok_or(CarrierError::NotCreature)?
            .death = Some(death);
        Ok(death)
    }
    /// Under the same exclusive physical owner turn as ordinary parent projection. A failure
    /// restores child HP/receipts before returning; no corpse/XP or durable workflow is created.
    fn cascade_native_summon_deaths(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        parent: ExactActorRef,
    ) -> Result<(), CarrierError> {
        let mut children = [None; 16];
        let mut count = 0;
        for link in self.native_summons.children(parent) {
            if count == 16 {
                return Err(CarrierError::CapacityExceeded);
            }
            children[count] = Some(link.child);
            count += 1;
        }
        if count == 0 {
            return Ok(());
        }
        let before = self.slots.clone();
        let mut deaths = Vec::new();
        deaths
            .try_reserve(count)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let result = (|| {
            for child in children.into_iter().flatten() {
                let index = match self.validate_summon_actor(continuity, child) {
                    Ok(i) => i,
                    Err(CarrierError::StaleActorGeneration) => continue,
                    Err(e) => return Err(e),
                };
                let Slot::CreatureOccupied {
                    health,
                    target_identity,
                    ..
                } = &self.slots[index]
                else {
                    return Err(CarrierError::NotCreature);
                };
                if *health > 0 {
                    let health = *health;
                    let target = copy_bounded_binding(target_identity)?;
                    let occurrence = format!(
                        "monster-master-death:{}:{}",
                        parent.0.actor_local_id.0, parent.0.actor_local_generation.0
                    );
                    let mut binding = occurrence.as_bytes().to_vec();
                    binding.extend_from_slice(b"\0MONSTER-SUMMON-1-master-lethal");
                    self.commit_creature_damage_inner(
                        continuity,
                        child.0,
                        OwnerDamageCommand {
                            target: &target,
                            occurrence: occurrence.as_bytes(),
                            binding: &binding,
                            damage: health,
                        },
                        None,
                        false,
                    )?;
                }
                let receipt = self.committed_lethal_receipt_inner(continuity, child.0)?;
                self.validate_lethal_receipt(continuity, &receipt)?;
                let p = receipt.projection.position.position;
                deaths.push(NativeSummonDeath {
                    child,
                    death: receipt.projection.occurrence.death_key(),
                    position: MovementLocalPosition {
                        x: p.x,
                        y: p.y,
                        floor: p.floor,
                    },
                });
            }
            Ok(())
        })();
        if let Err(e) = result {
            self.slots = before;
            return Err(e);
        }
        for death in deaths {
            if let Some(link) = self.native_summons.child_mut(death.child) {
                link.death = Some(death)
            }
        }
        Ok(())
    }
}

/// Bounded immutable physical admission input. Root content qualifier supplies source proof;
/// Foundation has no dependency on the project's loader or monster module.
#[derive(Debug, Clone)]
pub struct NativeSummonAdmissionSpec {
    parent: Box<str>,
    child: Box<str>,
    health: i64,
    index: usize,
    origin: NativeSummonOrigin,
    total: u32,
    count: u32,
    content: [u8; 32],
}
fn copy_native_summon_key(value: &str) -> Result<Box<str>, CarrierError> {
    if value.is_empty() || value.len() > MAX_OWNER_COMMIT_BINDING_BYTES {
        return Err(CarrierError::InvalidCreatureTarget);
    }
    let mut owned = String::new();
    owned
        .try_reserve_exact(value.len())
        .map_err(|_| CarrierError::CommitAllocationFailed)?;
    owned.push_str(value);
    Ok(owned.into_boxed_str())
}
impl NativeSummonAdmissionSpec {
    pub fn qualified(
        parent: &str,
        child: &str,
        health: i64,
        index: usize,
        total: u32,
        count: u32,
        content: [u8; 32],
    ) -> Result<Self, CarrierError> {
        if parent.is_empty()
            || child.is_empty()
            || health <= 0
            || index >= 8
            || total == 0
            || total > 16
            || count == 0
            || count > 16
        {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        Ok(Self {
            parent: copy_native_summon_key(parent)?,
            child: copy_native_summon_key(child)?,
            health,
            index,
            origin: NativeSummonOrigin::Behavior { index },
            total,
            count,
            content,
        })
    }
    pub fn parent_key(&self) -> &str {
        &self.parent
    }
    pub fn child_key(&self) -> &str {
        &self.child
    }
    pub fn health(&self) -> i64 {
        self.health
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn total(&self) -> u32 {
        self.total
    }
    pub fn count(&self) -> u32 {
        self.count
    }
    pub fn content(&self) -> [u8; 32] {
        self.content
    }
}

impl ChannelRuntimeV1 {
    /// Bounded native owner cleanup; physical slot retirement follows sealed child death.
    /// Retained history is a replay receipt, never authority to mutate a current slot.
    pub fn retire_native_summons(
        &mut self,
        current: &super::ScopeRuntimeFence,
        stamp: super::RuntimeWorkStamp,
    ) -> Result<usize, CarrierError> {
        let b = self.binding();
        if !current.is_current_for_scope(
            super::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !current.accepts_stamp(stamp)
        {
            return Err(CarrierError::WrongScope);
        }
        // A bounded pass (`SPAWNCEIL-RL-02`, Codex 4243039217): at most 256 links from where the
        // previous call stopped, wrapping, so any number of dead children is retired over later
        // calls without one call costing every link. It never refuses: the old fixed array of 64
        // refused the whole retirement, and with it every summon admission, once a 65th child was
        // dead (Codex 4242954089), and an admission retires its own parent's dead children.
        let (window, cursor) = self
            .carrier
            .native_summons
            .retirement_window(NATIVE_SUMMON_RETIREMENT_LINKS_PER_CALL_MAX)?;
        let mut dead = Vec::new();
        dead.try_reserve(window.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for child in window {
            let index = self
                .carrier
                .validate_summon_actor(&self.continuity, child)?;
            if matches!(
                &self.carrier.slots[index],
                Slot::CreatureOccupied { health: 0, .. }
            ) {
                dead.push(child);
            }
        }
        self.carrier.native_summons.retire_cursor = cursor;
        let count = dead.len();
        if count > 0 {
            self.carrier.retire_dead_summons(&self.continuity, dead)?;
        }
        Ok(count)
    }
}

impl ChannelRuntimeV1 {
    pub fn retire_native_summon_registration_failure(
        &mut self,
        current: &super::ScopeRuntimeFence,
        stamp: super::RuntimeWorkStamp,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        let b = self.binding();
        if !current.is_current_for_scope(
            super::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !current.accepts_stamp(stamp)
        {
            return Err(CarrierError::WrongScope);
        }
        if self.native_summon_role(actor)?.is_none() {
            return Err(CarrierError::NotCreature);
        }
        let index = self
            .carrier
            .validate_summon_actor(&self.continuity, actor)?;
        if matches!(
            &self.carrier.slots[index],
            Slot::CreatureOccupied { health: 0, .. }
        ) {
            self.carrier
                .project_native_summon_death_inner(&self.continuity, actor)?;
        }
        self.carrier.remove(&self.continuity, actor.0)?;
        Ok(())
    }
}
impl ChannelActorCarrier {
    fn summons_separated_by_move(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorRef,
        next: LocalPosition,
    ) -> Result<[Option<ExactActorRef>; 64], CarrierError> {
        let mut removed = [None; 64];
        let mut count = 0;
        for link in self.native_summons.touching(actor) {
            let child = self.validate_summon_actor(continuity, link.child)?;
            let parent = match self.validate_summon_actor(continuity, link.parent) {
                Ok(index) => index,
                Err(CarrierError::StaleActorGeneration) => {
                    removed[count] = Some(link.child);
                    count += 1;
                    continue;
                }
                Err(e) => return Err(e),
            };
            let position = |index: usize| match &self.slots[index] {
                Slot::CreatureOccupied {
                    position: Some(p), ..
                } => Ok(p.position),
                _ => Err(CarrierError::PositionUnavailable),
            };
            let child_at = if link.child.0 == actor {
                next
            } else {
                position(child)?
            };
            let parent_at = if link.parent.0 == actor {
                next
            } else {
                position(parent)?
            };
            if (i64::from(child_at.x) - i64::from(parent_at.x)).abs() > 30
                || (i64::from(child_at.y) - i64::from(parent_at.y)).abs() > 30
                || (i32::from(child_at.floor) - i32::from(parent_at.floor)).abs() > 2
            {
                if count == 64 {
                    return Err(CarrierError::CapacityExceeded);
                }
                removed[count] = Some(link.child);
                count += 1;
            }
        }
        Ok(removed)
    }
    fn remove_separated_summons(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        removed: [Option<ExactActorRef>; 64],
    ) -> Result<(), CarrierError> {
        for child in removed.into_iter().flatten() {
            let index = self.validate_summon_actor(continuity, child)?;
            if matches!(&self.slots[index], Slot::CreatureOccupied { health: 0, .. }) {
                self.project_native_summon_death_inner(continuity, child)?;
            }
            self.remove(continuity, child.0)?;
        }
        Ok(())
    }
}

/// Private source lifecycle receipt: a removed moving child has no published stale position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSummonMovementOutcome {
    Moved(MovementPositionSnapshot),
    Removed(ExactActorRef),
}
impl ChannelRuntimeV1 {
    pub fn commit_native_summon_cardinal(
        &mut self,
        current: &super::ScopeRuntimeFence,
        stamp: super::RuntimeWorkStamp,
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
        next: MovementLocalPosition,
    ) -> Result<NativeSummonMovementOutcome, CarrierError> {
        let b = self.binding();
        if !current.is_current_for_scope(
            super::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !current.accepts_stamp(stamp)
        {
            return Err(CarrierError::WrongScope);
        }
        let parent = self
            .native_summon_role(actor)?
            .ok_or(CarrierError::NotCreature)?;
        if !self.contains_live_creature(actor) || !self.contains_live_creature(parent) {
            return Err(CarrierError::CreatureNotActionable);
        }
        if expected.0.actor_ref != actor.0
            || self.read_actor_position(actor)? != expected
            || expected.context() != self.pinned_movement_context()
        {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        let at = expected.position();
        let dx = (i64::from(next.x) - i64::from(at.x)).abs();
        let dy = (i64::from(next.y) - i64::from(at.y)).abs();
        if next.floor != at.floor || dx + dy != 1 {
            return Err(CarrierError::MovementNonCardinal);
        }
        if !self.native_summon_cell_free(next) {
            return Err(CarrierError::CapacityExceeded);
        }
        let committed = self.carrier.compare_commit_position_with_summon_lifecycle(
            &self.continuity,
            expected.0,
            expected.0.version.context,
            LocalPosition {
                x: next.x,
                y: next.y,
                floor: next.floor,
            },
            true,
        )?;
        if self.native_summon_role(actor).is_err() {
            Ok(NativeSummonMovementOutcome::Removed(actor))
        } else {
            Ok(NativeSummonMovementOutcome::Moved(
                MovementPositionSnapshot(committed),
            ))
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
mod native_summon_reconciliation_tests {
    use super::*;
    fn fixture() -> ChannelRuntimeV1 {
        let id = |t: u8| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
        let w = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
        ChannelRuntimeV1::from_committed_assignment(
            w,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::test(w),
        )
        .expect("valid native fixture operation must succeed")
    }
    fn parent(r: &mut ChannelRuntimeV1) -> ExactActorRef {
        let a = r
            .carrier
            .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.rat", 20)
            .expect("valid native fixture operation must succeed");
        r.carrier
            .initialize_position(
                &r.continuity,
                a,
                r.pinned_position_context(),
                LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            )
            .expect("valid native fixture operation must succeed");
        ExactActorRef(a)
    }
    fn child(r: &mut ChannelRuntimeV1, p: ExactActorRef) -> ExactActorRef {
        let b = r.binding();
        let scope = super::super::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id());
        let mut f = super::super::ScopeRuntimeFence::from_external_grant(b.scope_generation())
            .with_scope(scope);
        let n = f
            .accept_input(b.scope_generation())
            .expect("valid native fixture operation must succeed");
        let stamp = f.stamp(n);
        let spec = NativeSummonAdmissionSpec::qualified(
            "oteryn:creature.rat",
            "oteryn:creature.rat",
            10,
            0,
            2,
            2,
            r.content.server_artifact_digest,
        )
        .expect("valid native fixture operation must succeed");
        r.admit_native_summon(
            &f,
            stamp,
            p,
            &spec,
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
        )
        .expect("valid native fixture operation must succeed")
    }
    #[test]
    fn separated_summon_late_reservation_failure_restores_occupied_index_and_visibility() {
        use super::super::{SourceActorReservationProof, source_reservation_seal};
        struct Proof {
            owner: ExactActorRef,
            session: GameSessionId,
            position: MovementPositionSnapshot,
            affected: [ExactActorRef; 1],
        }
        impl source_reservation_seal::Sealed for Proof {}
        impl SourceActorReservationProof for Proof {
            fn owner(&self) -> (ExactActorRef, GameSessionId) {
                (self.owner, self.session)
            }
            fn event_identity(&self) -> [u8; 16] {
                [7; 16]
            }
            fn source_binding_digest(&self) -> [u8; 32] {
                [8; 32]
            }
            fn affected_actors(&self) -> &[ExactActorRef] {
                &self.affected
            }
            fn current_for(&self, r: &ChannelRuntimeV1) -> bool {
                r.owner_fence().is_ok()
                    && r.read_actor_position(self.owner) == Ok(self.position)
                    && r.player_control_facts(self.owner, self.session)
                        .is_ok_and(|f| f.control_loss.is_none())
            }
        }
        let mut r = fixture();
        let p = parent(&mut r);
        let a = child(&mut r, p);
        let at = r
            .carrier
            .read_position(&r.continuity, a.0)
            .expect("valid native fixture operation must succeed");
        let context = r.pinned_position_context();
        r.carrier
            .compare_commit_position(
                &r.continuity,
                at,
                context,
                LocalPosition {
                    x: 102,
                    y: 100,
                    floor: 7,
                },
            )
            .expect("valid native fixture operation must succeed");
        let b = child(&mut r, p);
        let session =
            GameSessionId::decode(&[1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 4])
                .expect("valid native fixture operation must succeed");
        let reserved = r
            .reserve_fresh_session(session)
            .expect("valid native fixture operation must succeed");
        let owner = r
            .commit_fresh_session(reserved)
            .expect("valid native fixture operation must succeed");
        let position = match r
            .initialize_first_entry_position(owner)
            .expect("valid native fixture operation must succeed")
        {
            super::super::FirstEntryPosition::Initialized(p)
            | super::super::FirstEntryPosition::Reconciled(p) => p,
        };
        let proof = Proof {
            owner,
            session,
            position,
            affected: [b],
        };
        let mut reserved = r
            .prepare_source_actor_reservation(&proof)
            .expect("valid native fixture operation must succeed");
        r.reserve_source_actors(&mut reserved, &proof)
            .expect("valid native fixture operation must succeed");
        let slots = r.carrier.slots.clone();
        let free = r.carrier.free_head;
        let occupied = r.carrier.occupied.clone();
        let visible = r.visible_entities();
        let before = r
            .carrier
            .read_position(&r.continuity, p.0)
            .expect("valid native fixture operation must succeed");
        assert_eq!(
            r.carrier.compare_commit_position(
                &r.continuity,
                before,
                context,
                LocalPosition {
                    x: 200,
                    y: 100,
                    floor: 7
                }
            ),
            Err(CarrierError::PlanConflict)
        );
        assert_eq!(r.carrier.slots, slots);
        assert_eq!(r.carrier.free_head, free);
        assert_eq!(r.carrier.occupied, occupied);
        assert_eq!(r.visible_entities(), visible);
        assert!(r.contains_live_creature(a));
        assert!(r.contains_live_creature(b));
        // A later ordinary removal/re-admission uses a slot exactly once, with a new generation.
        r.carrier
            .remove(&r.continuity, a.0)
            .expect("valid native fixture operation must succeed");
        let reused = parent(&mut r);
        assert_eq!(reused.0.actor_local_id, a.0.actor_local_id);
        assert_ne!(reused.0.actor_local_generation, a.0.actor_local_generation);
        let index = u32::try_from(
            r.carrier
                .validate_ref(&r.continuity, reused.0)
                .expect("valid native fixture operation must succeed"),
        )
        .expect("valid native fixture operation must succeed");
        assert_eq!(
            r.carrier.occupied.iter().filter(|i| **i == index).count(),
            1
        );
    }
    #[test]
    fn parent_projection_cascades_native_death_and_preserves_main_condition_death_hook() {
        let mut r = fixture();
        let p = parent(&mut r);
        let a = child(&mut r, p);
        let i = r
            .carrier
            .validate_ref(&r.continuity, a.0)
            .expect("valid native fixture operation must succeed");
        let before = match &r.carrier.slots[i] {
            Slot::CreatureOccupied { committed, .. } => Some(committed.conditions.clone()),
            _ => None,
        }
        .expect("fixture must contain a native creature");
        r.carrier
            .current_owner_exact_commit(&r.continuity)
            .commit_damage(
                p,
                OwnerDamageCommand {
                    target: b"oteryn:creature.rat",
                    occurrence: b"master-lethal",
                    binding: b"master-lethal\0fixture",
                    damage: 20,
                },
            )
            .expect("valid native fixture operation must succeed");
        let mut death = r.borrow_combat_death();
        let receipt = death
            .committed_lethal_receipt(p)
            .expect("valid native fixture operation must succeed");
        death
            .project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
        assert!(!r.contains_live_creature(a));
        let after = match &r.carrier.slots[i] {
            Slot::CreatureOccupied { committed, .. } => Some(&committed.conditions),
            _ => None,
        }
        .expect("fixture must contain a native creature");
        assert_ne!(before, *after);
        let child_death = r
            .project_native_summon_death(a)
            .expect("valid native fixture operation must succeed");
        assert_eq!(child_death.child, a);
        assert_eq!(r.carrier.corpse_projections.len(), 1);
        assert_eq!(
            r.borrow_combat_death()
                .reward_occurrence(a, [0; 16])
                .expect_err("fixture must reject this invalid operation"),
            CarrierError::SummonHasNoRewards
        );
    }
    #[test]
    fn parent_administrative_remove_retires_child_without_corpse_and_generation_reuse_is_not_related()
     {
        let mut r = fixture();
        let p = parent(&mut r);
        let a = child(&mut r, p);
        r.carrier
            .remove(&r.continuity, p.0)
            .expect("valid native fixture operation must succeed");
        assert!(!r.contains_live_creature(a));
        assert!(r.carrier.native_summons.is_empty());
        assert!(r.carrier.corpse_projections.is_empty());
        assert!(r.carrier.death_reward_occurrences.is_empty());
        let replacement = parent(&mut r);
        assert_ne!(replacement, p);
        assert_eq!(r.native_summon_master(replacement), None);
    }
}

// Main reconciliation Foundation-3: source-qualified native boss helpers.
impl ExactActorRef {
    /// Current native actor slot ordinal for private source chain selection, never a wire ID.
    pub fn chain_local_actor_id(self) -> u64 {
        u64::from(self.0.actor_local_id.0)
    }
}
// Source-specific proposal appended inside foundation/runtime_actor_carrier.rs.
// Canary47dfd51f the_welter_heal.lua SHAead257f505dfcd8c6f2cd80bbaaff2185c0bc6e4e164e2d28d815ad328145cc6.
// No generic Encounter dispatcher. Owner lock/cast scheduling and emitted cues remain caller work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelterPrey {
    Egg,
    Spawn,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WelterConsumeResult {
    pub newly_committed: bool,
    pub consumed: Option<(ExactActorRef, WelterPrey)>,
    pub caster_position: MovementLocalPosition,
    pub prey_position: Option<MovementLocalPosition>,
    pub health_before: i64,
    pub health_after: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelterConsumeError {
    Carrier(CarrierError),
    InvalidSequence,
    SupersededOccurrence,
    WrongBossDefinition,
    InvalidBossHealth,
    MissingPosition,
    LedgerAllocation,
    RegistrationContentPinMismatch,
}
impl From<CarrierError> for WelterConsumeError {
    fn from(value: CarrierError) -> Self {
        Self::Carrier(value)
    }
}
#[derive(Debug, Clone, Copy)]
struct WelterConsumeEntry {
    caster: ExactActorRef,
    sequence: u64,
    result: WelterConsumeResult,
}
/// Beside ChannelRuntimeV1 under the same physical owner lock, exactly like CreatureBiteLedger.
/// At most one last occurrence per live caster, bounded by that carrier's actual slots.len().
#[derive(Debug, Default)]
pub struct WelterConsumeLedger {
    entries: Vec<WelterConsumeEntry>,
}
/// Constructed once by the native Content loader from current Creature profiles/bindings.
/// No token obtained from network data; private fields forbid caller struct construction.
#[derive(Debug, Clone, Copy)]
pub struct WelterSourceRegistration {
    world: WorldId,
    activation_sequence: u64,
    server_digest: [u8; 32],
    client_digest: [u8; 32],
    frame_digest: [u8; 32],
    map_digest: [u8; 32],
    max_health: i64,
}
impl ChannelRuntimeV1 {
    /// Content-loader seam: caller supplies values resolved from the three typed active profiles.
    /// The sourcebinding receipt is validated by that loader (Canary47dfd51f spell/monster SHA).
    /// Foundation accepts no generic profile graph and introduces no Content dependency.
    #[allow(clippy::too_many_arguments)]
    pub fn bind_welter_pinned_definition(
        &self,
        boss_key: &str,
        boss_name: &str,
        max_health: u64,
        egg_key: &str,
        egg_name: &str,
        spawn_key: &str,
        spawn_name: &str,
    ) -> Result<WelterSourceRegistration, WelterConsumeError> {
        if boss_key != "oteryn:creature.the_welter"
            || boss_name != "The Welter"
            || max_health != 25000
            || egg_key != "oteryn:creature.egg"
            || egg_name != "Egg"
            || spawn_key != "oteryn:creature.spawn_of_the_welter"
            || spawn_name != "Spawn of the Welter"
        {
            return Err(WelterConsumeError::WrongBossDefinition);
        }
        Ok(WelterSourceRegistration {
            world: self.content.world_id,
            activation_sequence: self.content.activation_sequence,
            server_digest: self.content.server_artifact_digest,
            client_digest: self.content.client_artifact_digest,
            frame_digest: self.content.frame_binding_digest,
            map_digest: self.content.map_revision_digest,
            max_health: max_health as i64,
        })
    }

    /// Native PROJECT approximation: stable carrier-slot spectator census order. Canary's
    /// supplied Game.getSpectators ordering is not specified by its spell. Explicit flag:
    /// PROJECT_NATIVE_SPECTATOR_ORDER_GLOBAL_UNVERIFIED. Not a claim of identical order.
    /// Exact source semantics otherwise: same floor ±10 square, first eligible live Monster,
    /// Egg/Spawn of the Welter, administrative removal (no death reward), heal25000 capped25000.
    pub fn commit_welter_consume(
        &mut self,
        ledger: &mut WelterConsumeLedger,
        registration: &WelterSourceRegistration,
        caster: ExactActorRef,
        sequence: u64,
    ) -> Result<WelterConsumeResult, WelterConsumeError> {
        self.commit_welter_consume_inner(ledger, registration, caster, sequence, false)
    }
    fn commit_welter_consume_inner(
        &mut self,
        ledger: &mut WelterConsumeLedger,
        registration: &WelterSourceRegistration,
        caster: ExactActorRef,
        sequence: u64,
        fail_before_write: bool,
    ) -> Result<WelterConsumeResult, WelterConsumeError> {
        if registration.world != self.content.world_id
            || registration.activation_sequence != self.content.activation_sequence
            || registration.server_digest != self.content.server_artifact_digest
            || registration.client_digest != self.content.client_artifact_digest
            || registration.frame_digest != self.content.frame_binding_digest
            || registration.map_digest != self.content.map_revision_digest
        {
            return Err(WelterConsumeError::RegistrationContentPinMismatch);
        }
        if sequence == 0 {
            return Err(WelterConsumeError::InvalidSequence);
        }
        let boss = self.carrier.validate_ref(&self.continuity, caster.0)?;
        self.carrier.assert_slot_spell_unreserved(boss)?;
        if !matches!(&self.carrier.slots[boss],Slot::CreatureOccupied{generation,..} if *generation==caster.0.actor_local_generation.0)
        {
            return Err(WelterConsumeError::Carrier(
                CarrierError::StaleActorGeneration,
            ));
        }
        let context = self.pinned_position_context();
        let (before, at) = match &self.carrier.slots[boss] {
            Slot::CreatureOccupied {
                target_identity,
                health,
                position: Some(p),
                ..
            } if target_identity.as_ref() == b"oteryn:creature.the_welter"
                && *health > 0
                && p.context == context =>
            {
                (*health, p.position)
            }
            Slot::CreatureOccupied {
                target_identity, ..
            } if target_identity.as_ref() != b"oteryn:creature.the_welter" => {
                return Err(WelterConsumeError::WrongBossDefinition);
            }
            Slot::CreatureOccupied { health, .. } if *health <= 0 => {
                return Err(WelterConsumeError::InvalidBossHealth);
            }
            _ => return Err(WelterConsumeError::MissingPosition),
        };
        // Source max health25000 is pinned to selected Creature data at registration.
        if before > registration.max_health {
            return Err(WelterConsumeError::InvalidBossHealth);
        }
        ledger
            .entries
            .retain(|e| self.contains_live_creature(e.caster));
        if let Some(e) = ledger.entries.iter().find(|e| e.caster == caster) {
            if sequence < e.sequence {
                return Err(WelterConsumeError::SupersededOccurrence);
            }
            if sequence == e.sequence {
                return Ok(WelterConsumeResult {
                    newly_committed: false,
                    ..e.result
                });
            }
        }
        let selected = self.carrier.slots.iter().enumerate().find_map(|(i, s)| {
            if i == boss {
                return None;
            }
            match s {
                Slot::CreatureOccupied {
                    generation,
                    target_identity,
                    health,
                    position: Some(p),
                    ..
                } if *health > 0
                    && p.context == context
                    && p.position.floor == at.floor
                    && i64::from(p.position.x).abs_diff(i64::from(at.x)) <= 10
                    && i64::from(p.position.y).abs_diff(i64::from(at.y)) <= 10 =>
                {
                    let prey = match target_identity.as_ref() {
                        b"oteryn:creature.egg" => WelterPrey::Egg,
                        b"oteryn:creature.spawn_of_the_welter" => WelterPrey::Spawn,
                        _ => return None,
                    };
                    Some((
                        ExactActorRef(ActorRef {
                            world_id: self.carrier.world_id,
                            channel_id: self.carrier.channel_id,
                            scope_generation: self.carrier.scope_generation,
                            actor_local_id: ActorLocalId((i + 1) as u32),
                            actor_local_generation: ActorLocalGeneration(*generation),
                        }),
                        prey,
                        p.position,
                    ))
                }
                _ => None,
            }
        });
        // All fallible allocation precedes either actor write. No fixed guessed ledger ceiling.
        if !ledger.entries.iter().any(|e| e.caster == caster) {
            if ledger.entries.len() >= self.carrier.slots.len() {
                return Err(WelterConsumeError::LedgerAllocation);
            }
            ledger
                .entries
                .try_reserve(1)
                .map_err(|_| WelterConsumeError::LedgerAllocation)?;
        }
        if fail_before_write {
            return Err(WelterConsumeError::Carrier(
                CarrierError::InjectedCommitFailure,
            ));
        }
        let after = if selected.is_some() {
            before.saturating_add(25000).min(registration.max_health)
        } else {
            before
        };
        if let Some((prey, _, _)) = selected {
            self.carrier.remove(&self.continuity, prey.0)?;
            if let Slot::CreatureOccupied { health, .. } = &mut self.carrier.slots[boss] {
                *health = after
            } else {
                unreachable!("same owner work item preserves boss slot")
            }
        }
        let result = WelterConsumeResult {
            newly_committed: true,
            consumed: selected.map(|(actor, prey, _)| (actor, prey)),
            caster_position: MovementLocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
            prey_position: selected.map(|(_, _, p)| MovementLocalPosition {
                x: p.x,
                y: p.y,
                floor: p.floor,
            }),
            health_before: before,
            health_after: after,
        };
        if let Some(e) = ledger.entries.iter_mut().find(|e| e.caster == caster) {
            *e = WelterConsumeEntry {
                caster,
                sequence,
                result,
            }
        } else {
            ledger.entries.push(WelterConsumeEntry {
                caster,
                sequence,
                result,
            })
        }
        Ok(result)
    }
}

impl CurrentOwnerCombatDeath<'_> {
    // Insert inside existing impl CurrentOwnerCombatDeath<'_>.
    // Actual carrier-owned data; neither method mints mutation authority or accepts caller facts.
    pub fn creature_target_identity(&self, actor: ExactActorRef) -> Result<&[u8], CarrierError> {
        let index = self.carrier.validate_ref(self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                target_identity, ..
            } => Ok(target_identity.as_ref()),
            _ => Err(CarrierError::NotCreature),
        }
    }
    pub fn damage_contributing_characters(
        &self,
        actor: ExactActorRef,
    ) -> Result<Vec<CharacterId>, CarrierError> {
        let index = self.carrier.validate_ref(self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                damage_contributors,
                ..
            } => {
                let mut result: Vec<CharacterId> = damage_contributors
                    .entries
                    .iter()
                    .map(|entry| entry.character)
                    .collect();
                result.sort();
                Ok(result)
            }
            _ => Err(CarrierError::NotCreature),
        }
    }
}
impl ChannelRuntimeV1 {
    // Insert inside impl ChannelRuntimeV1. Conservative exact-native source subset:
    // a unique player with no other live creature at its tile is unambiguously topCreature.
    // Ambiguous stack order is deliberately not invented; skipped tiles stay a runtime gap.
    pub fn smelly_unambiguous_targets(
        &self,
        caster: ExactActorRef,
    ) -> Result<Vec<(ExactActorRef, GameSessionId)>, CarrierError> {
        let caster_index = self.carrier.validate_ref(&self.continuity, caster.0)?;
        match &self.carrier.slots[caster_index] {
            Slot::CreatureOccupied {
                target_identity,
                health,
                ..
            } if target_identity.as_ref() == b"oteryn:creature.smelly_cheese" && *health > 0 => {}
            _ => return Err(CarrierError::NotCreature),
        }
        let at = self.carrier.read_position(&self.continuity, caster.0)?;
        let mut result = Vec::new();
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                let (Some(x), Some(y)) = (
                    at.version.position.x.checked_add(dx),
                    at.version.position.y.checked_add(dy),
                ) else {
                    continue;
                };
                let mut target = None;
                let mut count = 0;
                for (index, slot) in self.carrier.slots.iter().enumerate() {
                    let (version, generation, player) = match slot {
                        Slot::Occupied {
                            generation,
                            game_session_id: Some(session),
                            committed: true,
                            position: Some(version),
                            lifecycle,
                            ..
                        } => (
                            version,
                            *generation,
                            lifecycle.control_loss.is_none().then_some(*session),
                        ),
                        Slot::CreatureOccupied {
                            generation,
                            health,
                            position: Some(version),
                            ..
                        } if *health > 0 => (version, *generation, None),
                        _ => continue,
                    };
                    if version.context != at.version.context
                        || version.position.x != x
                        || version.position.y != y
                        || version.position.floor != at.version.position.floor
                    {
                        continue;
                    }
                    count += 1;
                    target = player.map(|session| {
                        (
                            ExactActorRef(ActorRef {
                                world_id: self.carrier.world_id,
                                channel_id: self.carrier.channel_id,
                                scope_generation: self.carrier.scope_generation,
                                actor_local_id: ActorLocalId((index + 1) as u32),
                                actor_local_generation: ActorLocalGeneration(generation),
                            }),
                            session,
                        )
                    });
                }
                if count == 1
                    && let Some(target) = target
                {
                    result.push(target);
                }
            }
        }
        Ok(result)
    }
}

// Qualification entry point only: not included in product proposal.
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[test]
fn welter_native_source_owner_proof() {
    fn id(tag: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = tag;
        b
    }
    fn runtime() -> ChannelRuntimeV1 {
        let w = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
        ChannelRuntimeV1::from_committed_assignment(
            w,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::from_activation(
                w,
                1,
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                (100, 100, 7),
            ),
        )
        .expect("valid native fixture operation must succeed")
    }
    fn actor(
        r: &mut ChannelRuntimeV1,
        target: &str,
        hp: i64,
        x: i32,
        y: i32,
        floor: i16,
    ) -> ExactActorRef {
        let a = r
            .carrier
            .admit_creature(&r.continuity, ActorState(1), target, hp)
            .expect("valid native fixture operation must succeed");
        let c = r.pinned_position_context();
        r.carrier
            .initialize_position(&r.continuity, a, c, LocalPosition { x, y, floor })
            .expect("valid native fixture operation must succeed");
        ExactActorRef(a)
    }
    let mut r = runtime();
    let b = actor(&mut r, "oteryn:creature.the_welter", 25_000, 100, 100, 7);
    let damage = r
        .carrier
        .current_owner_exact_commit(&r.continuity)
        .commit_damage(
            b,
            OwnerDamageCommand {
                target: b"oteryn:creature.the_welter",
                occurrence: b"source-test-damage",
                binding: b"source-test-damage\0hp-18000",
                damage: 18000,
            },
        )
        .expect("valid native fixture operation must succeed");
    assert_eq!(damage.health_after, 7000);
    let e = actor(&mut r, "oteryn:creature.egg", 100, 110, 110, 7);
    let s = actor(
        &mut r,
        "oteryn:creature.spawn_of_the_welter",
        100,
        100,
        100,
        7,
    );
    let values:serde_json::Value=serde_json::from_str(include_str!("../../../../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/encounters/welter-native-profiles.json")).expect("valid native fixture operation must succeed");
    let reg = r
        .bind_welter_pinned_definition(
            values[0]["target"]["key"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
            values[0]["data"]["profile"]["details"]["display_name"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
            values[0]["data"]["profile"]["health"]
                .as_u64()
                .expect("valid native fixture operation must succeed"),
            values[1]["target"]["key"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
            values[1]["data"]["profile"]["details"]["display_name"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
            values[2]["target"]["key"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
            values[2]["data"]["profile"]["details"]["display_name"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
        )
        .expect("valid native fixture operation must succeed");
    assert!(
        r.bind_welter_pinned_definition(
            "oteryn:creature.the_welter",
            "The Welter",
            25001,
            "oteryn:creature.egg",
            "Egg",
            "oteryn:creature.spawn_of_the_welter",
            "Spawn of the Welter"
        )
        .is_err()
    );
    let mut wrong_reg = reg;
    wrong_reg.server_digest[0] ^= 1;
    let snapshot = r.carrier.slots.clone();
    assert_eq!(
        r.commit_welter_consume(&mut WelterConsumeLedger::default(), &wrong_reg, b, 1),
        Err(WelterConsumeError::RegistrationContentPinMismatch)
    );
    assert_eq!(snapshot, r.carrier.slots);
    let mut ledger = WelterConsumeLedger::default();
    let before_failure = r.carrier.slots.clone();
    assert_eq!(
        r.commit_welter_consume_inner(&mut ledger, &reg, b, 1, true),
        Err(WelterConsumeError::Carrier(
            CarrierError::InjectedCommitFailure
        ))
    );
    assert_eq!(before_failure, r.carrier.slots);
    assert!(ledger.entries.is_empty());
    let a = r
        .commit_welter_consume(&mut ledger, &reg, b, 1)
        .expect("valid native fixture operation must succeed");
    assert_eq!(a.consumed, Some((e, WelterPrey::Egg)));
    assert_eq!((a.health_before, a.health_after), (7000, 25000));
    assert!(!r.contains_live_creature(e));
    assert_eq!(
        a.prey_position,
        Some(MovementLocalPosition {
            x: 110,
            y: 110,
            floor: 7
        })
    );
    assert!(r.carrier.corpse_projections.is_empty());
    assert!(r.carrier.death_reward_occurrences.is_empty());
    assert!(r.contains_live_creature(s));
    let replay = r
        .commit_welter_consume(&mut ledger, &reg, b, 1)
        .expect("valid native fixture operation must succeed");
    assert!(!replay.newly_committed);
    assert!(r.contains_live_creature(s));
    assert_eq!(
        r.commit_welter_consume(&mut ledger, &reg, b, 0),
        Err(WelterConsumeError::InvalidSequence)
    );
    let second = r
        .commit_welter_consume(&mut ledger, &reg, b, 2)
        .expect("valid native fixture operation must succeed");
    assert_eq!(second.consumed, Some((s, WelterPrey::Spawn)));
    assert_eq!(second.health_after, 25000);
    assert_eq!(
        r.commit_welter_consume(&mut ledger, &reg, b, 1),
        Err(WelterConsumeError::SupersededOccurrence)
    );
    let empty = r
        .commit_welter_consume(&mut ledger, &reg, b, 3)
        .expect("valid native fixture operation must succeed");
    assert!(empty.consumed.is_none());
    assert_eq!(empty.health_before, empty.health_after);
    let far = actor(&mut r, "oteryn:creature.egg", 100, 111, 100, 7);
    let upstairs = actor(&mut r, "oteryn:creature.egg", 100, 100, 100, 8);
    let no = r
        .commit_welter_consume(&mut ledger, &reg, b, 4)
        .expect("valid native fixture operation must succeed");
    assert!(no.consumed.is_none());
    assert!(r.contains_live_creature(far));
    assert!(r.contains_live_creature(upstairs));
    let wrong = actor(&mut r, "oteryn:creature.rat", 10, 100, 100, 7);
    let before = r.carrier.slots.clone();
    assert_eq!(
        r.commit_welter_consume(&mut ledger, &reg, wrong, 1),
        Err(WelterConsumeError::WrongBossDefinition)
    );
    assert_eq!(before, r.carrier.slots);
    let mut other = runtime();
    let foreign = actor(&mut other, "oteryn:creature.the_welter", 1, 100, 100, 7); // distinct assignment below
    other
        .continuity
        .advance(PreProductionContinuityGrant {
            world_id: other.carrier.world_id,
            channel_id: other.carrier.channel_id,
            scope_generation: ScopeOwnershipGeneration::new(2)
                .expect("valid native fixture operation must succeed"),
        })
        .expect("valid native fixture operation must succeed");
    let state = other.carrier.slots.clone();
    assert!(
        other
            .commit_welter_consume(&mut WelterConsumeLedger::default(), &reg, foreign, 1)
            .is_err()
    );
    assert_eq!(state, other.carrier.slots);
    r.carrier
        .remove(&r.continuity, b.0)
        .expect("valid native fixture operation must succeed");
    let replacement = actor(&mut r, "oteryn:creature.the_welter", 500, 100, 100, 7);
    assert_ne!(replacement, b);
    let state = r.carrier.slots.clone();
    assert!(r.commit_welter_consume(&mut ledger, &reg, b, 99).is_err());
    assert_eq!(state, r.carrier.slots);
    println!(
        "PASS actual ChannelRuntimeV1: first eligible/cap/one removal/replay/older/empty/range/floor/wrongidentity/retiredowner/stalegeneration"
    );
}
// Source-cue consumer reads the native committed table, never a caller-authored result.
impl ChannelRuntimeV1 {
    pub fn welter_committed_consume(
        &self,
        ledger: &WelterConsumeLedger,
        registration: &WelterSourceRegistration,
        caster: ExactActorRef,
        sequence: u64,
    ) -> Result<Option<(u64, WelterConsumeResult)>, WelterConsumeError> {
        if registration.world != self.content.world_id
            || registration.activation_sequence != self.content.activation_sequence
            || registration.server_digest != self.content.server_artifact_digest
            || registration.client_digest != self.content.client_artifact_digest
            || registration.frame_digest != self.content.frame_binding_digest
            || registration.map_digest != self.content.map_revision_digest
        {
            return Err(WelterConsumeError::RegistrationContentPinMismatch);
        }
        if !self.matches_live_creature_identity(caster, b"oteryn:creature.the_welter") {
            return Err(WelterConsumeError::Carrier(
                CarrierError::StaleActorGeneration,
            ));
        }
        Ok(ledger
            .entries
            .iter()
            .find(|e| e.caster == caster && e.sequence == sequence)
            .map(|e| (caster.0.actor_local_generation.0, e.result)))
    }
}
impl ChannelRuntimeV1 {
    /// Test-only native damage writer for source-specific lab fixtures. Target identity is
    /// read from the actual current slot; caller cannot supply an alternate target binding.
    #[cfg(any(test, feature = "test-support"))]
    pub fn commit_monster_lab_damage(
        &mut self,
        actor: ExactActorRef,
        occurrence: &[u8],
        damage: i64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let idx = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let target = match &self.carrier.slots[idx] {
            Slot::CreatureOccupied {
                generation,
                target_identity,
                ..
            } if *generation == actor.0.actor_local_generation.0 => target_identity.to_vec(),
            _ => return Err(CarrierError::StaleActorGeneration),
        };
        let mut binding = occurrence.to_vec();
        binding.extend_from_slice(b"\0monster-lab-native-damage-v1\0");
        binding.extend_from_slice(&damage.to_be_bytes());
        self.carrier
            .current_owner_exact_commit(&self.continuity)
            .commit_damage(
                actor,
                OwnerDamageCommand {
                    target: &target,
                    occurrence,
                    binding: &binding,
                    damage,
                },
            )
    }
}
impl ChannelRuntimeV1 {
    /// Test-only player geometry under its independently selected current ContentPin.
    /// Session identity and one-time initialization use the real carrier checks.
    #[cfg(any(test, feature = "test-support"))]
    pub fn initialize_source_pinned_lab_player_position(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        at: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .player_slot_index(&self.continuity, actor.0, session)?;
        let context = self.pinned_position_context();
        self.carrier
            .initialize_position(
                &self.continuity,
                actor.0,
                context,
                LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            )
            .map(MovementPositionSnapshot)
    }
    /// Test-only admission through the REAL current ContentPin position context. Existing
    /// movement-test fixture context is intentionally unchanged for its separate test contract.
    #[cfg(any(test, feature = "test-support"))]
    pub fn admit_source_pinned_lab_creature(
        &mut self,
        at: MovementLocalPosition,
        key: &str,
        health: i64,
    ) -> Result<ExactActorRef, CarrierError> {
        let context = self.pinned_position_context();
        let actor = self
            .carrier
            .admit_creature(&self.continuity, ActorState(0), key, health)?;
        self.carrier.initialize_position(
            &self.continuity,
            actor,
            context,
            LocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        )?;
        Ok(ExactActorRef(actor))
    }
}

// Source-specific approved authored Encounter state; NOT a Crystal Lua callback.
// Exact participant bindings are verified by the owning native Content loader.
const BONE_CAGE_KEYS: [&[u8]; 4] = [
    b"oteryn:creature.elyrax_s_soulcage",
    b"oteryn:creature.myzareth_s_soulcage",
    b"oteryn:creature.scarith_s_soulcage",
    b"oteryn:creature.zharvorin_s_soulcage",
];
const BONE_CASE_FINGERPRINT: &str =
    "cc3143372125a5bd4bff66bbd1dc0b3cf2989e6892e3135901f5083f2cd689ef";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoneOverlordPhase {
    Soulcages,
    Phylactery,
    Completed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BonePhaseError {
    Carrier(CarrierError),
    InvalidSource,
    WrongParticipant,
    WrongPhase,
    ContentChanged,
}
impl From<CarrierError> for BonePhaseError {
    fn from(e: CarrierError) -> Self {
        Self::Carrier(e)
    }
}
/// Fixed-size one-instance owner state: five captured native generations, four-bit death mask.
/// Does not implement shared HP, targetability, spawns, quest entry/exit or rewards.
#[derive(Debug)]
pub struct BoneOverlordCagePhase {
    cages: [ExactActorRef; 4],
    phylactery: ExactActorRef,
    dead_mask: u8,
    phase: BoneOverlordPhase,
    world: WorldId,
    channel: ChannelId,
    generation: ScopeOwnershipGeneration,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BonePhaseTransition {
    pub newly_committed: bool,
    pub cages_destroyed: u8,
    pub phase: BoneOverlordPhase,
}
impl ChannelRuntimeV1 {
    /// Native loader seam: registry has verified five canonical typed profiles and exact Crystal
    /// bindings. Fingerprint qualifies approved authored Encounter, not a donor callback claim.
    pub fn bind_bone_overlord_cage_phase(
        &self,
        cages: [ExactActorRef; 4],
        phylactery: ExactActorRef,
        encounter_fingerprint: &str,
        artifact_digest: [u8; 32],
    ) -> Result<BoneOverlordCagePhase, BonePhaseError> {
        if encounter_fingerprint != BONE_CASE_FINGERPRINT {
            return Err(BonePhaseError::InvalidSource);
        }
        if artifact_digest != self.content.server_artifact_digest {
            return Err(BonePhaseError::ContentChanged);
        }
        for (i, a) in cages.iter().enumerate() {
            if !self.matches_live_creature_identity(*a, BONE_CAGE_KEYS[i]) {
                return Err(BonePhaseError::WrongParticipant);
            }
            if cages[..i].contains(a) {
                return Err(BonePhaseError::WrongParticipant);
            }
        }
        if !self
            .matches_live_creature_identity(phylactery, b"oteryn:creature.bonelord_s_phylactery")
        {
            return Err(BonePhaseError::WrongParticipant);
        }
        Ok(BoneOverlordCagePhase {
            cages,
            phylactery,
            dead_mask: 0,
            phase: BoneOverlordPhase::Soulcages,
            world: self.content.world_id,
            channel: self.continuity.channel_id,
            generation: self.continuity.current_generation,
            activation: self.content.activation_sequence,
            server: self.content.server_artifact_digest,
            client: self.content.client_artifact_digest,
            frame: self.content.frame_binding_digest,
            map: self.content.map_revision_digest,
        })
    }
    /// Called after actual current-owner corpse projection, before despawn, under the same owner
    /// borrow. No caller-authored death boolean/key. Validation completes before phase write.
    pub fn commit_bone_overlord_projected_death(
        &mut self,
        state: &mut BoneOverlordCagePhase,
        actor: ExactActorRef,
    ) -> Result<BonePhaseTransition, BonePhaseError> {
        if state.world != self.content.world_id
            || state.channel != self.continuity.channel_id
            || state.generation != self.continuity.current_generation
            || state.activation != self.content.activation_sequence
            || state.server != self.content.server_artifact_digest
            || state.client != self.content.client_artifact_digest
            || state.frame != self.content.frame_binding_digest
            || state.map != self.content.map_revision_digest
        {
            return Err(BonePhaseError::ContentChanged);
        }
        let cage = state.cages.iter().position(|a| *a == actor);
        if cage.is_none() && actor != state.phylactery {
            return Err(BonePhaseError::WrongParticipant);
        }
        // validate_ref alone checks scope/index; explicitly reject a recycled actor generation.
        let idx = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let expected = if let Some(i) = cage {
            BONE_CAGE_KEYS[i]
        } else {
            b"oteryn:creature.bonelord_s_phylactery"
        };
        match &self.carrier.slots[idx] {
            Slot::CreatureOccupied {
                generation,
                target_identity,
                ..
            } if *generation == actor.0.actor_local_generation.0
                && target_identity.as_ref() == expected => {}
            _ => return Err(BonePhaseError::Carrier(CarrierError::StaleActorGeneration)),
        }
        self.carrier
            .current_owner_combat_death(&self.continuity)
            .projected_death(actor)?;
        let mut newly = false;
        if let Some(i) = cage {
            if state.dead_mask & (1 << i) == 0 {
                if state.phase != BoneOverlordPhase::Soulcages {
                    return Err(BonePhaseError::WrongPhase);
                }
                state.dead_mask |= 1 << i;
                newly = true;
                if state.dead_mask == 15 {
                    state.phase = BoneOverlordPhase::Phylactery
                }
            }
        } else if state.phase != BoneOverlordPhase::Completed {
            if state.phase != BoneOverlordPhase::Phylactery {
                return Err(BonePhaseError::WrongPhase);
            }
            state.phase = BoneOverlordPhase::Completed;
            newly = true;
        }
        Ok(BonePhaseTransition {
            newly_committed: newly,
            cages_destroyed: state.dead_mask.count_ones() as u8,
            phase: state.phase,
        })
    }
}

// Isolated proof: real current carrier, owner damage commits and corpse projections.
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[test]
fn bone_phase_actual_sealed_owner_deaths_native_proof() {
    fn id(tag: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = tag;
        b
    }
    let world = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
    let mut r = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
        NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        16,
        ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        ),
    )
    .expect("valid native fixture operation must succeed");
    fn actor(r: &mut ChannelRuntimeV1, key: &[u8], hp: i64) -> ExactActorRef {
        let a = r
            .carrier
            .admit_creature(
                &r.continuity,
                ActorState(1),
                std::str::from_utf8(key).expect("valid native fixture operation must succeed"),
                hp,
            )
            .expect("valid native fixture operation must succeed");
        let c = r.pinned_position_context();
        r.carrier
            .initialize_position(
                &r.continuity,
                a,
                c,
                LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            )
            .expect("valid native fixture operation must succeed");
        ExactActorRef(a)
    }
    fn kill(r: &mut ChannelRuntimeV1, a: ExactActorRef, key: &[u8]) {
        r.carrier
            .current_owner_exact_commit(&r.continuity)
            .commit_damage(
                a,
                OwnerDamageCommand {
                    target: key,
                    occurrence: b"source-cage-death",
                    binding: b"source-cage-death\0lethal",
                    damage: 120000,
                },
            )
            .expect("valid native fixture operation must succeed");
        let mut d = r.borrow_combat_death();
        let receipt = d
            .committed_lethal_receipt(a)
            .expect("valid native fixture operation must succeed");
        d.project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
    }
    let cages = BONE_CAGE_KEYS.map(|key| actor(&mut r, key, 120000));
    let phylactery = actor(&mut r, b"oteryn:creature.bonelord_s_phylactery", 50000);
    assert!(
        r.bind_bone_overlord_cage_phase(cages, phylactery, "wrong", [1; 32])
            .is_err()
    );
    assert!(
        r.bind_bone_overlord_cage_phase(cages, phylactery, BONE_CASE_FINGERPRINT, [9; 32])
            .is_err()
    );
    let mut bad = cages;
    bad[0] = cages[1];
    assert!(
        r.bind_bone_overlord_cage_phase(bad, phylactery, BONE_CASE_FINGERPRINT, [1; 32])
            .is_err()
    );
    let mut state = r
        .bind_bone_overlord_cage_phase(cages, phylactery, BONE_CASE_FINGERPRINT, [1; 32])
        .expect("valid native fixture operation must succeed");
    assert!(
        r.commit_bone_overlord_projected_death(&mut state, cages[0])
            .is_err()
    );
    assert_eq!(state.dead_mask, 0);
    let unrelated = actor(&mut r, b"oteryn:creature.cat", 10);
    assert_eq!(
        r.commit_bone_overlord_projected_death(&mut state, unrelated),
        Err(BonePhaseError::WrongParticipant)
    );
    for i in 0..4 {
        kill(&mut r, cages[i], BONE_CAGE_KEYS[i]);
        let x = r
            .commit_bone_overlord_projected_death(&mut state, cages[i])
            .expect("valid native fixture operation must succeed");
        assert!(x.newly_committed);
        assert_eq!(x.cages_destroyed, (i + 1) as u8);
        assert_eq!(
            x.phase,
            if i == 3 {
                BoneOverlordPhase::Phylactery
            } else {
                BoneOverlordPhase::Soulcages
            }
        );
        let duplicate = r
            .commit_bone_overlord_projected_death(&mut state, cages[i])
            .expect("valid native fixture operation must succeed");
        assert!(!duplicate.newly_committed);
        assert_eq!(duplicate.cages_destroyed, x.cages_destroyed);
    }
    kill(&mut r, phylactery, b"oteryn:creature.bonelord_s_phylactery");
    let victory = r
        .commit_bone_overlord_projected_death(&mut state, phylactery)
        .expect("valid native fixture operation must succeed");
    assert_eq!(victory.phase, BoneOverlordPhase::Completed);
    assert!(victory.newly_committed);
    assert!(
        !r.commit_bone_overlord_projected_death(&mut state, phylactery)
            .expect("valid native fixture operation must succeed")
            .newly_committed
    );
    // An old exact member cannot advance a new instance after its slot is reused.
    r.carrier
        .remove(&r.continuity, cages[0].0)
        .expect("valid native fixture operation must succeed");
    let replacement = actor(&mut r, BONE_CAGE_KEYS[0], 120000);
    assert_ne!(replacement, cages[0]);
    assert!(
        r.commit_bone_overlord_projected_death(&mut state, cages[0])
            .is_err()
    );
    // A source token is fenced by native content even when actor references look plausible.
    state.server = [9; 32];
    assert_eq!(
        r.commit_bone_overlord_projected_death(&mut state, cages[1]),
        Err(BonePhaseError::ContentChanged)
    );
    println!(
        "PASS Bone native4distinct sealed deaths/phase/duplicate/source/current-pin/stale-generation; full sharedHP/spawn/gates NOT CLAIMED"
    );
}

// Append beside native physical runtime_actor_carrier owner. No generic interpreter.
#[derive(Debug, Default)]
pub struct BorethDeathLedger {
    entries: Vec<CreatureDeathOccurrenceKey>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorethDeathResult {
    pub newly_committed: bool,
    pub removed: Vec<ExactActorRef>,
}
impl ChannelRuntimeV1 {
    /// Pinned Crystal 00ce02a5 BorethDeath: remove all exact Plaguethrower monsters in
    /// x32936..32944,y31474..31482,floor1; no caller role/name/position or reward authority.
    pub fn commit_boreth_death(
        &mut self,
        ledger: &mut BorethDeathLedger,
        boreth: ExactActorRef,
    ) -> Result<BorethDeathResult, CarrierError> {
        let owner = self.borrow_combat_death();
        if owner.creature_target_identity(boreth)? != b"oteryn:creature.boreth" {
            return Err(CarrierError::NotCreature);
        }
        let (death, _) = owner.projected_death(boreth)?;
        ledger.entries.retain(|key| {
            self.carrier.validate_ref(&self.continuity, key.0).is_ok()
                && self
                    .carrier
                    .corpse_projections
                    .iter()
                    .any(|p| p.occurrence.death_key() == *key)
        });
        if ledger.entries.contains(&death) {
            return Ok(BorethDeathResult {
                newly_committed: false,
                removed: Vec::new(),
            });
        }
        let context = self.pinned_position_context();
        let mut removed = Vec::new();
        removed
            .try_reserve(self.carrier.slots.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (i, slot) in self.carrier.slots.iter().enumerate() {
            if let Slot::CreatureOccupied {
                generation,
                target_identity,
                health,
                position: Some(p),
                ..
            } = slot
                && *health > 0
                && target_identity.as_ref() == b"oteryn:creature.plaguethrower"
                && p.context == context
                && p.position.floor == 1
                && (32936..=32944).contains(&p.position.x)
                && (31474..=31482).contains(&p.position.y)
            {
                removed.push(ExactActorRef(ActorRef {
                    world_id: self.carrier.world_id,
                    channel_id: self.carrier.channel_id,
                    scope_generation: self.carrier.scope_generation,
                    actor_local_id: ActorLocalId(
                        u32::try_from(i + 1)
                            .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                    ),
                    actor_local_generation: ActorLocalGeneration(*generation),
                }));
            }
        }
        // Preflight every fallible bound/ref/allocation before first physical owner mutation.
        ledger
            .entries
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        for actor in &removed {
            let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
            u32::try_from(index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        }
        for actor in &removed {
            self.carrier.remove(&self.continuity, actor.0)?;
        }
        ledger.entries.push(death);
        Ok(BorethDeathResult {
            newly_committed: true,
            removed,
        })
    }
}

#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[test]
fn crystal_boreth_native_proof() {
    fn id(tag: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = tag;
        b
    }
    let world = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
    let mut r = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
        NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        16,
        ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        ),
    )
    .expect("valid native fixture operation must succeed");
    fn actor(r: &mut ChannelRuntimeV1, key: &str, x: i32, y: i32, floor: u8) -> ExactActorRef {
        let a = r
            .carrier
            .admit_creature(&r.continuity, ActorState(0), key, 10)
            .expect("valid native fixture operation must succeed");
        r.carrier
            .initialize_position(
                &r.continuity,
                a,
                r.pinned_position_context(),
                LocalPosition {
                    x,
                    y,
                    floor: i16::from(floor),
                },
            )
            .expect("valid native fixture operation must succeed");
        ExactActorRef(a)
    }
    let boss = actor(&mut r, "oteryn:creature.boreth", 1, 1, 7);
    let a = actor(&mut r, "oteryn:creature.plaguethrower", 32936, 31474, 1);
    let b = actor(&mut r, "oteryn:creature.plaguethrower", 32944, 31482, 1);
    let outside = actor(&mut r, "oteryn:creature.plaguethrower", 32945, 31482, 1);
    let upstairs = actor(&mut r, "oteryn:creature.plaguethrower", 32940, 31477, 2);
    let rat = actor(&mut r, "oteryn:creature.rat", 32940, 31477, 1);
    let mut ledger = BorethDeathLedger::default();
    assert!(r.commit_boreth_death(&mut ledger, boss).is_err());
    assert!(r.contains_live_creature(a));
    r.carrier
        .current_owner_exact_commit(&r.continuity)
        .commit_damage(
            boss,
            OwnerDamageCommand {
                target: b"oteryn:creature.boreth",
                occurrence: b"boreth-death",
                binding: b"boreth-death\0hp10",
                damage: 10,
            },
        )
        .expect("valid native fixture operation must succeed");
    {
        let mut o = r.borrow_combat_death();
        let receipt = o
            .committed_lethal_receipt(boss)
            .expect("valid native fixture operation must succeed");
        o.project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
    }
    let corpses = r.carrier.corpse_projections.len();
    let rewards = r.carrier.death_reward_occurrences.len();
    let result = r
        .commit_boreth_death(&mut ledger, boss)
        .expect("valid native fixture operation must succeed");
    assert_eq!(result.removed, vec![a, b]);
    assert!(result.newly_committed);
    assert!(!r.contains_live_creature(a));
    assert!(r.contains_live_creature(outside));
    assert!(r.contains_live_creature(upstairs));
    assert!(r.contains_live_creature(rat));
    assert_eq!(r.carrier.corpse_projections.len(), corpses);
    assert_eq!(r.carrier.death_reward_occurrences.len(), rewards);
    let replacement = actor(&mut r, "oteryn:creature.plaguethrower", 32940, 31477, 1);
    assert!(
        !r.commit_boreth_death(&mut ledger, boss)
            .expect("valid native fixture operation must succeed")
            .newly_committed
    );
    assert!(r.contains_live_creature(replacement));
    assert!(r.commit_boreth_death(&mut ledger, rat).is_err());
    r.carrier
        .remove(&r.continuity, boss.0)
        .expect("valid native fixture operation must succeed");
    assert!(r.commit_boreth_death(&mut ledger, boss).is_err());
    assert!(r.contains_live_creature(replacement));
    println!(
        "PASS Boreth actual native committed death + administrative exact-rectangle removal; source guard, before-death guard, boundaries, floor, other species, no new corpses/rewards, retry replacement and stale generation"
    );
}

// Append in native physical runtime_actor_carrier. Conservative unambiguous topCreature subset.
#[derive(Debug, Default)]
pub struct RumBarrelDeathLedger {
    entries: Vec<RumBarrelDeathEntry>,
}
#[derive(Debug)]
struct RumBarrelDeathEntry {
    death: CreatureDeathOccurrenceKey,
    draw: i64,
    targets: Vec<(ExactActorRef, Option<OwnerDamageResult>)>,
}
#[derive(Debug, Clone)]
pub struct RumBarrelDeathResult {
    pub newly_committed: bool,
    pub targets: Vec<(ExactActorRef, OwnerDamageResult)>,
}
impl ChannelRuntimeV1 {
    /// Source Lua draws ONCE per death, shared across all 3x3 targets, 66666..88888 lifedrain.
    /// `server_draw` must be supplied by the current server RNG owner, never network input.
    /// Same-tile multi-actor stack authority is unavailable: ambiguous tiles are skipped and flagged.
    pub fn commit_rum_barrel_death(
        &mut self,
        ledger: &mut RumBarrelDeathLedger,
        barrel: ExactActorRef,
        server_draw: i64,
    ) -> Result<RumBarrelDeathResult, CarrierError> {
        if !(66666..=88888).contains(&server_draw) {
            return Err(CarrierError::InvalidDamage);
        }
        let owner = self.borrow_combat_death();
        if owner.creature_target_identity(barrel)? != b"oteryn:creature.rum_barrel" {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let (death, center) = owner.projected_death(barrel)?;
        ledger.entries.retain(|e| {
            self.carrier
                .validate_ref(&self.continuity, e.death.0)
                .is_ok()
                && self
                    .carrier
                    .corpse_projections
                    .iter()
                    .any(|p| p.occurrence.death_key() == e.death)
        });
        let existed = ledger.entries.iter().any(|e| e.death == death);
        let entry = if let Some(index) = ledger.entries.iter().position(|e| e.death == death) {
            if ledger.entries[index].draw != server_draw {
                return Err(CarrierError::PlanConflict);
            }
            index
        } else {
            let context = self.pinned_position_context();
            let mut targets = Vec::new();
            targets
                .try_reserve(9)
                .map_err(|_| CarrierError::AllocationFailed)?;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let mut count = 0;
                    let mut target = None;
                    for (i, slot) in self.carrier.slots.iter().enumerate() {
                        let position = match slot {
                            Slot::Occupied {
                                position: Some(p), ..
                            }
                            | Slot::CreatureOccupied {
                                position: Some(p),
                                health: 1..,
                                ..
                            } => p,
                            _ => continue,
                        };
                        if position.context != context
                            || position.position.floor != center.floor
                            || i64::from(position.position.x) != i64::from(center.x) + dx
                            || i64::from(position.position.y) != i64::from(center.y) + dy
                        {
                            continue;
                        }
                        count += 1;
                        if let Slot::CreatureOccupied {
                            generation,
                            target_identity,
                            ..
                        } = slot
                            && target_identity.as_ref() == b"oteryn:creature.weak_spot"
                        {
                            target = Some(ExactActorRef(ActorRef {
                                world_id: self.carrier.world_id,
                                channel_id: self.carrier.channel_id,
                                scope_generation: self.carrier.scope_generation,
                                actor_local_id: ActorLocalId(
                                    u32::try_from(i + 1)
                                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                                ),
                                actor_local_generation: ActorLocalGeneration(*generation),
                            }));
                        }
                    }
                    if count == 1
                        && let Some(target) = target
                    {
                        targets.push((target, None));
                    }
                }
            }
            ledger
                .entries
                .try_reserve(1)
                .map_err(|_| CarrierError::AllocationFailed)?;
            ledger.entries.push(RumBarrelDeathEntry {
                death,
                draw: server_draw,
                targets,
            });
            ledger.entries.len() - 1
        };
        let newly_committed = !existed
            || ledger.entries[entry]
                .targets
                .iter()
                .any(|(_, result)| result.is_none());
        // Once-per-death producer fixes the target census; retries cannot damage replacement actors.
        // Native per-target receipt remains authoritative if a later allocation fails mid callback.
        let mut completed = Vec::new();
        completed
            .try_reserve_exact(ledger.entries[entry].targets.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (target, result) in &mut ledger.entries[entry].targets {
            let committed = if let Some(committed) = *result {
                committed
            } else {
                let occurrence = format!(
                    "rum:{}:{}:{}",
                    death.actor_local_id(),
                    death.actor_local_generation(),
                    target.0.actor_local_id.0
                );
                let binding = format!("{occurrence}\0crystal-rum-lifedrain:{server_draw}");
                let committed = self
                    .carrier
                    .current_owner_exact_commit(&self.continuity)
                    .commit_damage(
                        *target,
                        OwnerDamageCommand {
                            target: b"oteryn:creature.weak_spot",
                            occurrence: occurrence.as_bytes(),
                            binding: binding.as_bytes(),
                            damage: server_draw,
                        },
                    )?;
                *result = Some(committed);
                committed
            };
            // Result storage was reserved before the first target HP write.
            completed.push((*target, committed));
        }
        Ok(RumBarrelDeathResult {
            newly_committed,
            targets: completed,
        })
    }
}

#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[test]
fn crystal_rum_native_proof() {
    fn id(tag: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = tag;
        b
    }
    let world = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
    let mut r = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
        NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        16,
        ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        ),
    )
    .expect("valid native fixture operation must succeed");
    fn actor(
        r: &mut ChannelRuntimeV1,
        key: &str,
        health: i64,
        x: i32,
        y: i32,
        floor: i16,
    ) -> ExactActorRef {
        let a = r
            .carrier
            .admit_creature(&r.continuity, ActorState(0), key, health)
            .expect("valid native fixture operation must succeed");
        r.carrier
            .initialize_position(
                &r.continuity,
                a,
                r.pinned_position_context(),
                LocalPosition { x, y, floor },
            )
            .expect("valid native fixture operation must succeed");
        ExactActorRef(a)
    }
    let boss = actor(&mut r, "oteryn:creature.rum_barrel", 10, 100, 100, 7);
    let a = actor(&mut r, "oteryn:creature.weak_spot", 200000, 99, 99, 7);
    let b = actor(&mut r, "oteryn:creature.weak_spot", 200000, 101, 101, 7);
    let upstairs = actor(&mut r, "oteryn:creature.weak_spot", 200000, 101, 101, 8);
    let outside = actor(&mut r, "oteryn:creature.weak_spot", 200000, 102, 100, 7);
    let overlapped = actor(&mut r, "oteryn:creature.weak_spot", 200000, 99, 100, 7);
    let rat = actor(&mut r, "oteryn:creature.rat", 10, 99, 100, 7);
    let mut ledger = RumBarrelDeathLedger::default();
    assert!(r.commit_rum_barrel_death(&mut ledger, boss, 70000).is_err());
    r.carrier
        .current_owner_exact_commit(&r.continuity)
        .commit_damage(
            boss,
            OwnerDamageCommand {
                target: b"oteryn:creature.rum_barrel",
                occurrence: b"rum-death",
                binding: b"rum-death\0hp10",
                damage: 10,
            },
        )
        .expect("valid native fixture operation must succeed");
    {
        let mut o = r.borrow_combat_death();
        let receipt = o
            .committed_lethal_receipt(boss)
            .expect("valid native fixture operation must succeed");
        o.project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
    }
    assert!(r.commit_rum_barrel_death(&mut ledger, boss, 66665).is_err());
    assert!(ledger.entries.is_empty());
    let result = r
        .commit_rum_barrel_death(&mut ledger, boss, 70000)
        .expect("valid native fixture operation must succeed");
    assert!(result.newly_committed);
    assert_eq!(result.targets.len(), 2);
    for (_, damage) in result.targets {
        assert_eq!(
            (damage.health_before, damage.health_after),
            (200000, 130000)
        );
    }
    assert!(r.contains_live_creature(upstairs));
    assert!(r.contains_live_creature(outside));
    assert!(r.contains_live_creature(overlapped));
    assert!(r.contains_live_creature(rat));
    let repeat = r
        .commit_rum_barrel_death(&mut ledger, boss, 70000)
        .expect("valid native fixture operation must succeed");
    assert!(!repeat.newly_committed);
    assert_eq!(repeat.targets[0].1.health_after, 130000);
    assert_eq!(
        r.commit_rum_barrel_death(&mut ledger, boss, 70001)
            .expect_err("fixture must reject this invalid operation"),
        CarrierError::PlanConflict
    );
    r.carrier
        .remove(&r.continuity, a.0)
        .expect("valid native fixture operation must succeed");
    let replacement = actor(&mut r, "oteryn:creature.weak_spot", 200000, 99, 99, 7);
    assert!(
        !r.commit_rum_barrel_death(&mut ledger, boss, 70000)
            .expect("valid native fixture operation must succeed")
            .newly_committed
    );
    let idx = r
        .carrier
        .validate_ref(&r.continuity, replacement.0)
        .expect("valid native fixture operation must succeed");
    assert!(matches!(
        &r.carrier.slots[idx],
        Slot::CreatureOccupied { health: 200000, .. }
    ));
    assert!(r.commit_rum_barrel_death(&mut ledger, b, 70000).is_err());
    println!(
        "PASS Rum real owner HP commit shared one source draw, death/definition/range/floor/ambiguous-stack guards, replay and changed draw conflict, replacement generation untouched"
    );
}

// Append beside native physical carrier. Ten exact source callbacks, not an Encounter interpreter.
#[derive(Debug, Default)]
pub struct TentacleDeathLedger {
    entries: Vec<(CreatureDeathOccurrenceKey, Option<ExactActorRef>)>,
}
#[derive(Debug, Clone, Copy)]
pub struct TentacleNextResult {
    pub newly_committed: bool,
    pub spawned: Option<ExactActorRef>,
    pub position: Option<MovementLocalPosition>,
}
fn crystal_tentacle_line(key: &[u8]) -> Result<&'static [MovementLocalPosition], CarrierError> {
    Ok(match key {
        b"oteryn:creature.tentacle" => &[
            MovementLocalPosition {
                x: 33723,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33724,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33725,
                y: 31180,
                floor: 7,
            },
        ],
        b"oteryn:creature.tentacle2" => &[
            MovementLocalPosition {
                x: 33723,
                y: 31186,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33723,
                y: 31187,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33723,
                y: 31188,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33723,
                y: 31189,
                floor: 7,
            },
        ],
        b"oteryn:creature.tentacle3" => &[
            MovementLocalPosition {
                x: 33727,
                y: 31184,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33727,
                y: 31185,
                floor: 7,
            },
        ],
        b"oteryn:creature.tentacle4" => &[
            MovementLocalPosition {
                x: 33731,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33732,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33733,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33734,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33735,
                y: 31180,
                floor: 7,
            },
        ],
        b"oteryn:creature.tentacle5" => &[
            MovementLocalPosition {
                x: 33718,
                y: 31180,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33718,
                y: 31179,
                floor: 7,
            },
            MovementLocalPosition {
                x: 33718,
                y: 31178,
                floor: 7,
            },
        ],
        b"oteryn:creature.tentacle6" => &[
            MovementLocalPosition {
                x: 33714,
                y: 31180,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33714,
                y: 31179,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33714,
                y: 31178,
                floor: 6,
            },
        ],
        b"oteryn:creature.tentacle7" => &[
            MovementLocalPosition {
                x: 33726,
                y: 31182,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33726,
                y: 31181,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33726,
                y: 31180,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33726,
                y: 31179,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33726,
                y: 31178,
                floor: 6,
            },
        ],
        b"oteryn:creature.tentacle8" => &[
            MovementLocalPosition {
                x: 33718,
                y: 31187,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33718,
                y: 31188,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33718,
                y: 31189,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33718,
                y: 31190,
                floor: 6,
            },
        ],
        b"oteryn:creature.tentacle9" => &[
            MovementLocalPosition {
                x: 33714,
                y: 31185,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33713,
                y: 31185,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33712,
                y: 31185,
                floor: 6,
            },
        ],
        b"oteryn:creature.tentacle10" => &[
            MovementLocalPosition {
                x: 33716,
                y: 31181,
                floor: 6,
            },
            MovementLocalPosition {
                x: 33716,
                y: 31182,
                floor: 6,
            },
        ],
        _ => return Err(CarrierError::CreatureTargetMismatch),
    })
}
impl ChannelRuntimeV1 {
    /// Callable only after the root source-specific Map owner removes the line prop.
    /// Data-loader caller must qualify current exact source profile health2500 and map frame.
    /// Creates the existing same species through native admission+initialize; no synthetic identity.
    /// Final-tile transform/removal and ENERGYHIT remain independent WorldObject/presentation work.
    pub fn commit_tentacle_next(
        &mut self,
        ledger: &mut TentacleDeathLedger,
        actor: ExactActorRef,
    ) -> Result<TentacleNextResult, CarrierError> {
        let owner = self.borrow_combat_death();
        let key = owner.creature_target_identity(actor)?;
        let line = crystal_tentacle_line(key)?;
        let identity = std::str::from_utf8(key)
            .map_err(|_| CarrierError::CreatureTargetMismatch)?
            .to_owned();
        let (death, position) = owner.projected_death(actor)?;
        let next = line
            .iter()
            .position(|p| *p == position)
            .map_or(0, |index| index + 1);
        let at = line.get(next).copied();
        ledger.entries.retain(|(key, _)| {
            self.carrier.validate_ref(&self.continuity, key.0).is_ok()
                && self
                    .carrier
                    .corpse_projections
                    .iter()
                    .any(|p| p.occurrence.death_key() == *key)
        });
        if let Some((_, spawned)) = ledger.entries.iter().find(|(key, _)| *key == death) {
            return Ok(TentacleNextResult {
                newly_committed: false,
                spawned: *spawned,
                position: at,
            });
        }
        ledger
            .entries
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let spawned = if let Some(at) = at {
            let context = self.pinned_position_context();
            let created =
                self.carrier
                    .admit_creature(&self.continuity, ActorState(0), &identity, 2500)?;
            if let Err(error) = self.carrier.initialize_position(
                &self.continuity,
                created,
                context,
                LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            ) {
                self.carrier.remove(&self.continuity, created)?;
                return Err(error);
            }
            Some(ExactActorRef(created))
        } else {
            None
        };
        ledger.entries.push((death, spawned));
        Ok(TentacleNextResult {
            newly_committed: true,
            spawned,
            position: at,
        })
    }
}

#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[test]
fn crystal_tentacle_native_proof() {
    fn id(tag: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = tag;
        b
    }
    fn runtime() -> ChannelRuntimeV1 {
        let world = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
        ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            ChannelContentPin::from_activation(
                world,
                1,
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                (100, 100, 7),
            ),
        )
        .expect("valid native fixture operation must succeed")
    }
    fn dead(r: &mut ChannelRuntimeV1, key: &str, at: MovementLocalPosition) -> ExactActorRef {
        let a = r
            .carrier
            .admit_creature(&r.continuity, ActorState(0), key, 2500)
            .expect("valid native fixture operation must succeed");
        r.carrier
            .initialize_position(
                &r.continuity,
                a,
                r.pinned_position_context(),
                LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            )
            .expect("valid native fixture operation must succeed");
        let a = ExactActorRef(a);
        r.carrier
            .current_owner_exact_commit(&r.continuity)
            .commit_damage(
                a,
                OwnerDamageCommand {
                    target: key.as_bytes(),
                    occurrence: b"tentacle-death",
                    binding: b"tentacle-death\0source2500",
                    damage: 2500,
                },
            )
            .expect("valid native fixture operation must succeed");
        let mut o = r.borrow_combat_death();
        let receipt = o
            .committed_lethal_receipt(a)
            .expect("valid native fixture operation must succeed");
        o.project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
        a
    }
    for n in 1..=10 {
        let key = if n == 1 {
            "oteryn:creature.tentacle".to_owned()
        } else {
            format!("oteryn:creature.tentacle{n}")
        };
        let line = crystal_tentacle_line(key.as_bytes())
            .expect("valid native fixture operation must succeed");
        assert!(line.len() >= 2);
        let mut r = runtime();
        let a = dead(&mut r, &key, line[0]);
        let mut ledger = TentacleDeathLedger::default();
        let result = r
            .commit_tentacle_next(&mut ledger, a)
            .expect("valid native fixture operation must succeed");
        assert!(result.newly_committed);
        assert_eq!(result.position, Some(line[1]));
        let created = result
            .spawned
            .expect("valid native fixture operation must succeed");
        assert_eq!(
            r.read_actor_position(created)
                .expect("valid native fixture operation must succeed")
                .position(),
            line[1]
        );
        let index = r
            .carrier
            .validate_ref(&r.continuity, created.0)
            .expect("valid native fixture operation must succeed");
        assert!(
            matches!(&r.carrier.slots[index],Slot::CreatureOccupied{health:2500,target_identity,..} if target_identity.as_ref()==key.as_bytes())
        );
        assert!(
            !r.commit_tentacle_next(&mut ledger, a)
                .expect("valid native fixture operation must succeed")
                .newly_committed
        );
        let final_actor = dead(
            &mut r,
            &key,
            *line
                .last()
                .expect("valid native fixture operation must succeed"),
        );
        let result = r
            .commit_tentacle_next(&mut ledger, final_actor)
            .expect("valid native fixture operation must succeed");
        assert!(result.newly_committed);
        assert!(result.spawned.is_none());
        assert!(result.position.is_none());
        let fallback = dead(
            &mut r,
            &key,
            MovementLocalPosition {
                x: 1,
                y: 1,
                floor: 1,
            },
        );
        let result = r
            .commit_tentacle_next(&mut ledger, fallback)
            .expect("valid native fixture operation must succeed");
        assert_eq!(result.position, Some(line[0]));
        assert_eq!(
            r.read_actor_position(
                result
                    .spawned
                    .expect("valid native fixture operation must succeed")
            )
            .expect("valid native fixture operation must succeed")
            .position(),
            line[0]
        );
        r.carrier
            .remove(&r.continuity, a.0)
            .expect("valid native fixture operation must succeed");
        assert!(r.commit_tentacle_next(&mut ledger, a).is_err());
    }
    assert!(crystal_tentacle_line(b"oteryn:creature.rat").is_err());
    println!(
        "PASS all10 Tentacle actual native sealed-death spawn-next fresh generation+HP2500, exact ordered/reversed lines, final no-spawn, outside-line first fallback, repeated death and retired source guard"
    );
}

impl ChannelRuntimeV1 {
    /// Read the existing current carrier, not timer enrollment. The explicit candidate budget
    /// refuses a partial census; geometry/hostility qualification still belongs to perception.
    pub fn current_live_creature_candidates(
        &self,
        limit: usize,
    ) -> Result<Vec<ExactActorRef>, CarrierError> {
        let context = self.pinned_position_context();
        let mut out = Vec::new();
        for (i, slot) in self.carrier.slots.iter().enumerate() {
            let Slot::CreatureOccupied {
                generation,
                health,
                position: Some(position),
                ..
            } = slot
            else {
                continue;
            };
            if *health <= 0 {
                continue;
            }
            if position.context != context {
                return Err(CarrierError::PositionContextMismatch);
            }
            if out.len() >= limit {
                return Err(CarrierError::AllocationFailed);
            }
            out.try_reserve(1)
                .map_err(|_| CarrierError::AllocationFailed)?;
            out.push(ExactActorRef(ActorRef {
                world_id: self.carrier.world_id,
                channel_id: self.carrier.channel_id,
                scope_generation: self.carrier.scope_generation,
                actor_local_id: ActorLocalId((i + 1) as u32),
                actor_local_generation: ActorLocalGeneration(*generation),
            }));
        }
        Ok(out)
    }
    /// Position Combat targets every creature on the tile. This bounded self subset is legal
    /// only when the actual carrier proves no OTHER positioned actor occupies that cell.
    /// Unqualified/dead/temporarily uncontrolled other actors conservatively veto, never vanish.
    pub fn is_sole_current_actor_on_cell(
        &self,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let snapshot = self.read_actor_position(actor)?;
        if snapshot.context() != self.pinned_movement_context() {
            return Err(CarrierError::PositionContextMismatch);
        }
        let cell = snapshot.position();
        for (i, slot) in self.carrier.slots.iter().enumerate() {
            if i == index {
                continue;
            }
            let p = match slot {
                Slot::Occupied {
                    position: Some(p), ..
                }
                | Slot::CreatureOccupied {
                    position: Some(p), ..
                } => p,
                _ => continue,
            };
            if p.position.x == cell.x && p.position.y == cell.y && p.position.floor == cell.floor {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

// Append in physical Foundation carrier. Pure test-only owner construction; no production grants.
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
pub fn crystal_death_router_fixture(world: WorldId) -> ChannelRuntimeV1 {
    crystal_death_router_fixture_digest(world, [1; 32])
}
#[cfg(any(test, feature = "test-support"))]
pub fn crystal_death_router_fixture_with_artifact(
    world: WorldId,
    digest: [u8; 32],
) -> ChannelRuntimeV1 {
    crystal_death_router_fixture_digest(world, digest)
}
#[cfg(any(test, feature = "test-support"))]
#[allow(clippy::expect_used)]
fn crystal_death_router_fixture_digest(world: WorldId, digest: [u8; 32]) -> ChannelRuntimeV1 {
    let mut channel = [0u8; 16];
    channel[6] = 0x70;
    channel[8] = 0x80;
    channel[15] = 92;
    let mut node = channel;
    node[15] = 93;
    ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&channel).expect("valid native fixture operation must succeed"),
        NodeId::decode(&node).expect("valid native fixture operation must succeed"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        16,
        ChannelContentPin::from_activation(
            world,
            1,
            digest,
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        ),
    )
    .expect("valid native fixture operation must succeed")
}
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
impl ChannelRuntimeV1 {
    pub fn crystal_router_fixture_actor(
        &mut self,
        key: &str,
        health: i64,
        at: MovementLocalPosition,
    ) -> ExactActorRef {
        let actor = self
            .carrier
            .admit_creature(&self.continuity, ActorState(0), key, health)
            .expect("valid native fixture operation must succeed");
        self.carrier
            .initialize_position(
                &self.continuity,
                actor,
                self.pinned_position_context(),
                LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            )
            .expect("valid native fixture operation must succeed");
        ExactActorRef(actor)
    }
    pub fn crystal_router_fixture_project_death(
        &mut self,
        actor: ExactActorRef,
        key: &str,
        health: i64,
    ) {
        self.carrier
            .current_owner_exact_commit(&self.continuity)
            .commit_damage(
                actor,
                OwnerDamageCommand {
                    target: key.as_bytes(),
                    occurrence: b"router-fixture-death",
                    binding: b"router-fixture-death\0fixture",
                    damage: health,
                },
            )
            .expect("valid native fixture operation must succeed");
        let mut owner = self.borrow_combat_death();
        let receipt = owner
            .committed_lethal_receipt(actor)
            .expect("valid native fixture operation must succeed");
        owner
            .project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
    }
    pub fn crystal_router_fixture_health(&self, actor: ExactActorRef) -> i64 {
        let index = self
            .carrier
            .validate_ref(&self.continuity, actor.0)
            .expect("valid native fixture operation must succeed");
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied { health, .. } => Some(*health),
            _ => None,
        }
        .expect("fixture requires native creature")
    }
}

#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
impl ChannelRuntimeV1 {
    pub fn crystal_router_fixture_commit_lethal(
        &mut self,
        actor: ExactActorRef,
        key: &str,
        health: i64,
    ) {
        self.carrier
            .current_owner_exact_commit(&self.continuity)
            .commit_damage(
                actor,
                OwnerDamageCommand {
                    target: key.as_bytes(),
                    occurrence: b"router-hook-fixture-death",
                    binding: b"router-hook-fixture-death\0fixture",
                    damage: health,
                },
            )
            .expect("valid native fixture operation must succeed");
    }
}

// PROJECT approximation only: wiki shared-life semantics + Crystal per-cage120000 baseline.
// The physical creature slots are the only HP values; registration/phase state stores no pool HP.
#[derive(Debug)]
pub struct BoneSharedHpResult {
    pub transitions: [OwnerDamageResult; 4],
    pub approximation: &'static str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeBoneSharedGroup {
    cages: [ExactActorRef; 4],
    phylactery: ExactActorRef,
    world: WorldId,
    channel: ChannelId,
    generation: ScopeOwnershipGeneration,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    cage_deaths: [Option<CreatureDeathOccurrenceKey>; 4],
}
impl ChannelRuntimeV1 {
    /// Called only after existing actual five-profile Bone registry qualified the phase instance.
    /// No health counter is stored: all shared health lives in the four physical creature slots.
    pub fn register_bone_shared_hp(
        &mut self,
        state: &BoneOverlordCagePhase,
        current: &super::ScopeRuntimeFence,
        stamp: super::RuntimeWorkStamp,
    ) -> Result<(), BonePhaseError> {
        let indexes = self.bone_shared_owner_guard(state, current, stamp)?;
        if indexes.iter().any(|i| {
            !matches!(
                self.carrier.slots[*i],
                Slot::CreatureOccupied { health: 1.., .. }
            )
        }) {
            return Err(BonePhaseError::WrongPhase);
        }
        if self
            .carrier
            .bone_shared_groups
            .iter()
            .any(|g| g.cages == state.cages && g.phylactery == state.phylactery)
        {
            return Ok(());
        }
        if self.carrier.bone_shared_groups.iter().any(|g| {
            g.cages.iter().any(|a| state.cages.contains(a)) || g.phylactery == state.phylactery
        }) {
            return Err(BonePhaseError::WrongParticipant);
        }
        let keep: [bool; 16] = std::array::from_fn(|i| {
            self.carrier.bone_shared_groups.get(i).is_some_and(|g|g.cages.iter().chain(std::iter::once(&g.phylactery)).any(|a|self.carrier.validate_ref(&self.continuity,a.0).ok().is_some_and(|index|matches!(&self.carrier.slots[index],Slot::CreatureOccupied{generation,..}if *generation==a.0.actor_local_generation.0))))
        });
        let mut n = 0;
        self.carrier.bone_shared_groups.retain(|_| {
            let retain = keep[n];
            n += 1;
            retain
        });
        if self.carrier.bone_shared_groups.len() >= 16 {
            return Err(BonePhaseError::Carrier(CarrierError::CapacityExceeded));
        }
        self.carrier
            .bone_shared_groups
            .try_reserve(1)
            .map_err(|_| BonePhaseError::Carrier(CarrierError::AllocationFailed))?;
        self.carrier.bone_shared_groups.push(NativeBoneSharedGroup {
            cages: state.cages,
            phylactery: state.phylactery,
            world: state.world,
            channel: state.channel,
            generation: state.generation,
            activation: state.activation,
            server: state.server,
            client: state.client,
            frame: state.frame,
            map: state.map,
            cage_deaths: [None; 4],
        });
        Ok(())
    }
    fn bone_shared_pin_matches(&self, g: &NativeBoneSharedGroup) -> bool {
        g.world == self.content.world_id
            && g.channel == self.continuity.channel_id
            && g.generation == self.continuity.current_generation
            && g.activation == self.content.activation_sequence
            && g.server == self.content.server_artifact_digest
            && g.client == self.content.client_artifact_digest
            && g.frame == self.content.frame_binding_digest
            && g.map == self.content.map_revision_digest
    }
}
impl ChannelRuntimeV1 {
    fn bone_shared_owner_guard(
        &self,
        state: &BoneOverlordCagePhase,
        current: &super::ScopeRuntimeFence,
        stamp: super::RuntimeWorkStamp,
    ) -> Result<[usize; 4], BonePhaseError> {
        let b = self.binding();
        let scope = super::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id());
        if !current.is_current_for_scope(scope, b.scope_generation())
            || !current.accepts_stamp(stamp)
        {
            return Err(BonePhaseError::Carrier(CarrierError::WrongScope));
        }
        if state.world != self.content.world_id
            || state.channel != self.continuity.channel_id
            || state.generation != self.continuity.current_generation
            || state.activation != self.content.activation_sequence
            || state.server != self.content.server_artifact_digest
            || state.client != self.content.client_artifact_digest
            || state.frame != self.content.frame_binding_digest
            || state.map != self.content.map_revision_digest
        {
            return Err(BonePhaseError::ContentChanged);
        }
        if state.phase != BoneOverlordPhase::Soulcages || state.dead_mask != 0 {
            return Err(BonePhaseError::WrongPhase);
        }
        let mut indexes = [0; 4];
        let mut hp = None;
        for (i, actor) in state.cages.iter().enumerate() {
            let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
            let Slot::CreatureOccupied {
                health,
                target_identity,
                ..
            } = &self.carrier.slots[index]
            else {
                return Err(BonePhaseError::WrongParticipant);
            };
            if target_identity.as_ref() != BONE_CAGE_KEYS[i]
                || self.native_summon_role(*actor)?.is_some()
                || *health < 0
                || *health > 120000
                || indexes[..i].contains(&index)
            {
                return Err(BonePhaseError::WrongParticipant);
            }
            if hp.is_some_and(|prior| prior != *health) {
                return Err(BonePhaseError::InvalidSource);
            }
            hp = Some(*health);
            indexes[i] = index;
        }
        Ok(indexes)
    }
    /// Same generic native receipt/contributor preparation as ordinary damage, staged for four
    /// actual actors before any physical HP write. One incoming occurrence, separate per-actor
    /// receipts; replay/changed-plan refusals retain ordinary native semantics.
    // Keep commit_bone_shared_damage ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_bone_shared_damage(
        &mut self,
        state: &BoneOverlordCagePhase,
        current: &super::ScopeRuntimeFence,
        stamp: super::RuntimeWorkStamp,
        target: ExactActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
    ) -> Result<BoneSharedHpResult, BonePhaseError> {
        let indexes = self.bone_shared_owner_guard(state, current, stamp)?;
        let target_index = state
            .cages
            .iter()
            .position(|a| *a == target)
            .ok_or(BonePhaseError::WrongParticipant)?;
        if command.target != BONE_CAGE_KEYS[target_index] {
            return Err(BonePhaseError::WrongParticipant);
        }
        let mut staged: [Option<Slot>; 4] = [None, None, None, None];
        let mut results = [OwnerDamageResult {
            applied: false,
            health_before: 0,
            health_after: 0,
        }; 4];
        for i in 0..4 {
            let (index, next, result) = self.carrier.prepare_creature_damage_inner(
                &self.continuity,
                state.cages[i].0,
                OwnerDamageCommand {
                    target: BONE_CAGE_KEYS[i],
                    occurrence: command.occurrence,
                    binding: command.binding,
                    damage: command.damage,
                },
                attacker,
                fail_before_write,
            )?;
            if index != indexes[i] {
                return Err(BonePhaseError::WrongParticipant);
            }
            staged[i] = next;
            results[i] = result;
        }
        if results.iter().any(|r| {
            r.applied != results[0].applied
                || r.health_before != results[0].health_before
                || r.health_after != results[0].health_after
        }) {
            return Err(BonePhaseError::InvalidSource);
        }
        if self.bone_shared_owner_guard(state, current, stamp)? != indexes {
            return Err(BonePhaseError::WrongParticipant);
        }
        // All fallible work and guards completed. Exclusive current-owner publication, no callbacks.
        for (index, next) in indexes.into_iter().zip(staged) {
            if let Some(next) = next {
                self.carrier.slots[index] = next
            }
        }
        Ok(BoneSharedHpResult {
            transitions: results,
            approximation: "PROJECT_SHARED_POOL_FROM_CRYSTAL_CAGE_MAX_120000_NOT_GLOBAL",
        })
    }
}

impl ChannelActorCarrier {
    fn note_bone_shared_projection(
        &mut self,
        actor: ExactActorRef,
        death: CreatureDeathOccurrenceKey,
    ) {
        for g in &mut self.bone_shared_groups {
            if let Some(i) = g.cages.iter().position(|a| *a == actor) {
                g.cage_deaths[i] = Some(death)
            }
        }
    }
    fn bone_shared_indexes(
        &self,
        c: &NamespaceContinuityGuard,
        g: &NativeBoneSharedGroup,
    ) -> Result<[usize; 4], CarrierError> {
        if self.world_id != g.world
            || self.channel_id != g.channel
            || self.scope_generation != g.generation
        {
            return Err(CarrierError::WrongScope);
        }
        let mut indexes = [0; 4];
        let mut hp = None;
        for i in 0..4 {
            let index = self.validate_ref(c, g.cages[i].0)?;
            let Slot::CreatureOccupied {
                generation,
                health,
                target_identity,
                ..
            } = &self.slots[index]
            else {
                return Err(CarrierError::NotCreature);
            };
            if *generation != g.cages[i].0.actor_local_generation.0 {
                return Err(CarrierError::StaleActorGeneration);
            }
            if target_identity.as_ref() != BONE_CAGE_KEYS[i]
                || *health < 0
                || *health > 120000
                || indexes[..i].contains(&index)
                || self.is_native_summon(g.cages[i])
            {
                return Err(CarrierError::CreatureTargetMismatch);
            }
            if hp.is_some_and(|old| old != *health) {
                return Err(CarrierError::PlanConflict);
            }
            hp = Some(*health);
            indexes[i] = index;
        }
        Ok(indexes)
    }
    #[allow(clippy::too_many_arguments)]
    fn commit_bone_shared_damage_inner(
        &mut self,
        c: &NamespaceContinuityGuard,
        g: &NativeBoneSharedGroup,
        target: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail: bool,
        max_sub_ordinal: Option<u16>,
        now_ms: Option<u64>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let indexes = self.bone_shared_indexes(c, g)?;
        let selected = g
            .cages
            .iter()
            .position(|a| a.0 == target)
            .ok_or(CarrierError::CreatureTargetMismatch)?;
        if command.target != BONE_CAGE_KEYS[selected] {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let mut slots: [Option<Slot>; 4] = [None, None, None, None];
        let mut out = [OwnerDamageResult {
            applied: false,
            health_before: 0,
            health_after: 0,
        }; 4];
        for i in 0..4 {
            let (index, next, result) = self.prepare_creature_damage_inner_bounded(
                c,
                g.cages[i].0,
                OwnerDamageCommand {
                    target: BONE_CAGE_KEYS[i],
                    occurrence: command.occurrence,
                    binding: command.binding,
                    damage: command.damage,
                },
                attacker,
                fail,
                max_sub_ordinal,
                false,
                now_ms,
            )?;
            if index != indexes[i] {
                return Err(CarrierError::StaleActorGeneration);
            }
            slots[i] = next;
            out[i] = result;
        }
        if out.iter().any(|r| {
            r.applied != out[0].applied
                || r.health_before != out[0].health_before
                || r.health_after != out[0].health_after
        }) {
            return Err(CarrierError::PlanConflict);
        }
        if self.bone_shared_indexes(c, g)? != indexes {
            return Err(CarrierError::StaleActorGeneration);
        }
        for (index, slot) in indexes.into_iter().zip(slots) {
            if let Some(slot) = slot {
                self.slots[index] = slot
            }
        }
        Ok(out[selected])
    }
}
impl ChannelRuntimeV1 {
    /// Normalize inside existing source-qualified native heal sink, before its preflight/publication.
    /// PROJECT coalescence: one source cast applies once per shared group; lowest source roster index
    /// in requested targets supplies its qualified draw. No invented spirit spell/radius/threshold.
    fn expand_bone_shared_heals(
        &self,
        requests: &[(ExactActorRef, String, u64, u64)],
    ) -> Result<NativeSourceHealProjection, CarrierError> {
        let mut expanded = Vec::new();
        expanded
            .try_reserve(64)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut map = Vec::new();
        map.try_reserve(requests.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (n, (actor, key, max, draw)) in requests.iter().enumerate() {
            if *max == 0 || *max > i64::MAX as u64 || *draw > i64::MAX as u64 {
                return Err(CarrierError::InvalidCreatureHealth);
            }
            if requests[..n].iter().any(|r| r.0 == *actor) {
                return Err(CarrierError::PlanConflict);
            }
            if let Some(g) = self
                .carrier
                .bone_shared_groups
                .iter()
                .find(|g| g.cages.contains(actor))
            {
                if !self.bone_shared_pin_matches(g) {
                    return Err(CarrierError::PositionContextMismatch);
                }
                let indexes = self.carrier.bone_shared_indexes(&self.continuity, g)?;
                let selected = g
                    .cages
                    .iter()
                    .position(|a| a == actor)
                    .ok_or(CarrierError::CreatureTargetMismatch)?;
                if key.as_bytes() != BONE_CAGE_KEYS[selected] || *max != 120000 {
                    return Err(CarrierError::CreatureTargetMismatch);
                }
                let chosen = g
                    .cages
                    .iter()
                    .find_map(|a| requests.iter().find(|r| r.0 == *a))
                    .ok_or(CarrierError::PlanConflict)?;
                if !expanded
                    .iter()
                    .any(|r: &(ExactActorRef, String, u64, u64)| g.cages.contains(&r.0))
                {
                    if expanded.len() + 4 > 64 {
                        return Err(CarrierError::AllocationFailed);
                    }
                    for i in 0..4 {
                        let Slot::CreatureOccupied { health, .. } = self.carrier.slots[indexes[i]]
                        else {
                            return Err(CarrierError::NotCreature);
                        };
                        if health <= 0 {
                            return Err(CarrierError::CreatureNotActionable);
                        }
                        expanded.push((
                            g.cages[i],
                            std::str::from_utf8(BONE_CAGE_KEYS[i])
                                .map_err(|_| CarrierError::InvalidCreatureTarget)?
                                .to_owned(),
                            120000,
                            chosen.3,
                        ));
                    }
                }
                map.push(
                    expanded
                        .iter()
                        .position(|r| r.0 == *actor)
                        .ok_or(CarrierError::PlanConflict)?,
                );
            } else {
                if expanded.len() >= 64 {
                    return Err(CarrierError::AllocationFailed);
                }
                map.push(expanded.len());
                expanded.push((*actor, key.clone(), *max, *draw));
            }
        }
        Ok((expanded, map))
    }
}

// Test-only native owner harness. Registration callback supplied by root test reads actual project.
// No Content dependency is introduced into standalone Foundation path includes.
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
pub fn bone_shared_actual_owner_harness(
    world: WorldId,
    artifact_digest: [u8; 32],
    mut register: impl FnMut(
        &ChannelRuntimeV1,
        [ExactActorRef; 4],
        ExactActorRef,
    ) -> BoneOverlordCagePhase,
) {
    fn id(t: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = t;
        b
    }
    fn runtime(world: super::WorldId, artifact_digest: [u8; 32]) -> ChannelRuntimeV1 {
        ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            32,
            ChannelContentPin::from_activation(
                world,
                1,
                artifact_digest,
                [2; 32],
                [3; 32],
                [4; 32],
                (100, 100, 7),
            ),
        )
        .expect("valid native fixture operation must succeed")
    }
    fn actor(r: &mut ChannelRuntimeV1, k: &[u8], hp: i64) -> ExactActorRef {
        let a = r
            .carrier
            .admit_creature(
                &r.continuity,
                ActorState(0),
                std::str::from_utf8(k).expect("valid native fixture operation must succeed"),
                hp,
            )
            .expect("valid native fixture operation must succeed");
        r.carrier
            .initialize_position(
                &r.continuity,
                a,
                r.pinned_position_context(),
                LocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            )
            .expect("valid native fixture operation must succeed");
        ExactActorRef(a)
    }
    fn hp(r: &ChannelRuntimeV1, a: ExactActorRef) -> i64 {
        let i = r
            .carrier
            .validate_ref(&r.continuity, a.0)
            .expect("valid native fixture operation must succeed");
        match r.carrier.slots[i] {
            Slot::CreatureOccupied { health, .. } => Some(health),
            _ => None,
        }
        .expect("fixture must contain a native creature")
    }
    fn damage(
        r: &mut ChannelRuntimeV1,
        a: ExactActorRef,
        key: &[u8],
        occ: &[u8],
        binding: &[u8],
        n: i64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        r.borrow_exact_actor_commit().commit_damage(
            a,
            OwnerDamageCommand {
                target: key,
                occurrence: occ,
                binding,
                damage: n,
            },
        )
    }
    let mut r = runtime(world, artifact_digest);
    let cages = BONE_CAGE_KEYS.map(|k| actor(&mut r, k, 120000));
    let phyl = actor(&mut r, b"oteryn:creature.bonelord_s_phylactery", 50000);
    let mut state = register(&r, cages, phyl);
    let scope = super::RuntimeScopeRefV1::channel(world, r.binding().channel_id());
    let generation = r.binding().scope_generation();
    let mut fence = super::ScopeRuntimeFence::from_external_grant(generation).with_scope(scope);
    let ordinal = fence
        .accept_input(generation)
        .expect("valid native fixture operation must succeed");
    let stamp = fence.stamp(ordinal);

    let mut wrong = [0; 16];
    wrong[6] = 0x70;
    wrong[8] = 0x80;
    wrong[15] = 99;
    let wrong = super::ScopeRuntimeFence::from_external_grant(generation).with_scope(
        super::RuntimeScopeRefV1::channel(
            world,
            ChannelId::decode(&wrong).expect("valid native fixture operation must succeed"),
        ),
    );
    assert!(r.register_bone_shared_hp(&state, &wrong, stamp).is_err());
    r.register_bone_shared_hp(&state, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    assert_eq!(r.carrier.bone_shared_groups.len(), 1);
    r.register_bone_shared_hp(&state, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    assert_eq!(r.carrier.bone_shared_groups.len(), 1);
    assert_eq!(
        damage(
            &mut r,
            phyl,
            b"oteryn:creature.bonelord_s_phylactery",
            b"premature",
            b"premature\0hit",
            50000
        ),
        Err(CarrierError::CreatureNotActionable)
    );
    assert_eq!(hp(&r, phyl), 50000);
    let first = damage(&mut r, cages[1], BONE_CAGE_KEYS[1], b"A", b"A\0hit400", 400)
        .expect("valid native fixture operation must succeed");
    assert!(first.applied);
    for a in cages {
        assert_eq!(hp(&r, a), 119600)
    }
    assert!(
        !damage(&mut r, cages[1], BONE_CAGE_KEYS[1], b"A", b"A\0hit400", 400)
            .expect("valid native fixture operation must succeed")
            .applied
    );
    assert!(
        damage(
            &mut r,
            cages[1],
            BONE_CAGE_KEYS[1],
            b"A",
            b"A\0changed",
            500
        )
        .is_err()
    );
    for a in cages {
        assert_eq!(hp(&r, a), 119600)
    }
    let heals = vec![
        (
            cages[2],
            std::str::from_utf8(BONE_CAGE_KEYS[2])
                .expect("valid native fixture operation must succeed")
                .to_owned(),
            120000,
            200,
        ),
        (
            cages[0],
            std::str::from_utf8(BONE_CAGE_KEYS[0])
                .expect("valid native fixture operation must succeed")
                .to_owned(),
            120000,
            100,
        ),
    ];
    let healed = r
        .commit_source_creature_heal_batch(artifact_digest, &heals)
        .expect("valid native fixture operation must succeed");
    assert_eq!(healed.len(), 2);
    for a in cages {
        assert_eq!(hp(&r, a), 119700)
    } // source-roster lowest index chosen, oncecast
    let duplicate = vec![heals[0].clone(), heals[0].clone()];
    assert!(
        r.commit_source_creature_heal_batch(artifact_digest, &duplicate)
            .is_err()
    );
    for a in cages {
        assert_eq!(hp(&r, a), 119700)
    }
    r.content.map_revision_digest[0] ^= 1;
    assert!(
        damage(
            &mut r,
            cages[0],
            BONE_CAGE_KEYS[0],
            b"generic-map",
            b"generic-map\0hit",
            100
        )
        .is_err()
    );
    assert!(
        damage(
            &mut r,
            phyl,
            b"oteryn:creature.bonelord_s_phylactery",
            b"generic-phyl-map",
            b"generic-phyl-map\0hit",
            1
        )
        .is_err()
    );
    assert!(
        r.commit_source_creature_heal_batch(artifact_digest, &heals)
            .is_err()
    );
    assert!(
        r.commit_bone_shared_damage(
            &state,
            &fence,
            stamp,
            cages[0],
            OwnerDamageCommand {
                target: BONE_CAGE_KEYS[0],
                occurrence: b"map",
                binding: b"map\0hit",
                damage: 100
            },
            None,
            false
        )
        .is_err()
    );
    for a in cages {
        assert_eq!(hp(&r, a), 119700)
    }
    r.content.map_revision_digest[0] ^= 1;
    damage(
        &mut r,
        cages[0],
        BONE_CAGE_KEYS[0],
        b"lethal",
        b"lethal\0hp",
        120000,
    )
    .expect("valid native fixture operation must succeed");
    for a in cages {
        assert_eq!(hp(&r, a), 0)
    }
    assert!(
        r.commit_source_creature_heal_batch(artifact_digest, &heals)
            .is_err()
    );
    for (i, a) in cages.into_iter().enumerate() {
        let mut owner = r.borrow_combat_death();
        let receipt = owner
            .committed_lethal_receipt(a)
            .expect("valid native fixture operation must succeed");
        owner
            .project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
        let projected = owner
            .projected_death(a)
            .expect("valid native fixture operation must succeed")
            .0;
        assert_eq!(projected.actor_local_id(), a.0.actor_local_id.0);
        let phase = r
            .commit_bone_overlord_projected_death(&mut state, a)
            .expect("valid native fixture operation must succeed");
        assert_eq!(phase.cages_destroyed, (i + 1) as u8);
        if i < 3 {
            assert!(
                damage(
                    &mut r,
                    phyl,
                    b"oteryn:creature.bonelord_s_phylactery",
                    b"early",
                    b"early\0hit",
                    1
                )
                .is_err()
            );
        }
    }
    assert_eq!(state.phase, BoneOverlordPhase::Phylactery);
    assert!(
        r.carrier.bone_shared_groups[0]
            .cage_deaths
            .iter()
            .all(Option::is_some)
    );
    damage(
        &mut r,
        phyl,
        b"oteryn:creature.bonelord_s_phylactery",
        b"victory",
        b"victory\0hit",
        50000,
    )
    .expect("valid native fixture operation must succeed");
    {
        let mut owner = r.borrow_combat_death();
        let receipt = owner
            .committed_lethal_receipt(phyl)
            .expect("valid native fixture operation must succeed");
        owner
            .project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
    }
    assert_eq!(
        r.commit_bone_overlord_projected_death(&mut state, phyl)
            .expect("valid native fixture operation must succeed")
            .phase,
        BoneOverlordPhase::Completed
    );
    assert_eq!(r.carrier.corpse_projections.len(), 5);
    assert!(r.carrier.death_reward_occurrences.is_empty());
    // Late fourth-slot conflict: three prepared native slots must remain unpublished.
    let mut q = runtime(world, artifact_digest);
    let c = BONE_CAGE_KEYS.map(|k| actor(&mut q, k, 120000));
    let p = actor(&mut q, b"oteryn:creature.bonelord_s_phylactery", 50000);
    let st = register(&q, c, p);
    for i in 0..3 {
        damage(&mut q, c[i], BONE_CAGE_KEYS[i], b"seed", b"seed\0hit", 400)
            .expect("valid native fixture operation must succeed");
    }
    damage(
        &mut q,
        c[3],
        BONE_CAGE_KEYS[3],
        b"late",
        b"late\0hit400",
        400,
    )
    .expect("valid native fixture operation must succeed");
    q.register_bone_shared_hp(&st, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    let before = q.carrier.slots.clone();
    assert_eq!(
        damage(
            &mut q,
            c[0],
            BONE_CAGE_KEYS[0],
            b"late",
            b"late\0hit200",
            200
        ),
        Err(CarrierError::PlanConflict)
    );
    assert_eq!(q.carrier.slots, before);
    q.carrier
        .remove(&q.continuity, c[1].0)
        .expect("valid native fixture operation must succeed");
    let replacement = actor(&mut q, BONE_CAGE_KEYS[1], 120000);
    let before = q.carrier.slots.clone();
    assert!(
        damage(
            &mut q,
            c[0],
            BONE_CAGE_KEYS[0],
            b"stale",
            b"stale\0hit",
            200
        )
        .is_err()
    );
    assert_eq!(q.carrier.slots, before);
    assert_ne!(replacement, c[1]);
    // Same source species and identical physical HP: only exact local generation can refuse fanout.
    let mut replacement_case = runtime(world, artifact_digest);
    let rc = BONE_CAGE_KEYS.map(|k| actor(&mut replacement_case, k, 120000));
    let rp = actor(
        &mut replacement_case,
        b"oteryn:creature.bonelord_s_phylactery",
        50000,
    );
    let rs = register(&replacement_case, rc, rp);
    replacement_case
        .register_bone_shared_hp(&rs, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    damage(
        &mut replacement_case,
        rc[0],
        BONE_CAGE_KEYS[0],
        b"before-replacement",
        b"before-replacement\0damage400",
        400,
    )
    .expect("valid native fixture operation must succeed");
    replacement_case
        .carrier
        .remove(&replacement_case.continuity, rc[1].0)
        .expect("valid native fixture operation must succeed");
    let reused = actor(&mut replacement_case, BONE_CAGE_KEYS[1], 119600);
    assert_eq!(reused.0.actor_local_id, rc[1].0.actor_local_id);
    assert_ne!(
        reused.0.actor_local_generation,
        rc[1].0.actor_local_generation
    );
    let before = replacement_case.carrier.slots.clone();
    assert_eq!(
        damage(
            &mut replacement_case,
            rc[0],
            BONE_CAGE_KEYS[0],
            b"replacement-fanout",
            b"replacement-fanout\0damage200",
            200
        ),
        Err(CarrierError::StaleActorGeneration)
    );
    assert_eq!(replacement_case.carrier.slots, before);
    let heal = vec![(
        rc[0],
        std::str::from_utf8(BONE_CAGE_KEYS[0])
            .expect("valid native fixture operation must succeed")
            .to_owned(),
        120000,
        100,
    )];
    assert_eq!(
        replacement_case.commit_source_creature_heal_batch(artifact_digest, &heal),
        Err(CarrierError::StaleActorGeneration)
    );
    assert_eq!(replacement_case.carrier.slots, before);
    assert_eq!(hp(&replacement_case, reused), 119600);
    // Retired exact generations must not consume the16-group bound forever. New same-species
    // occupants in reused indexes are not the retired group's members. Keep a partially live group.
    let mut pruning = runtime(world, artifact_digest);
    let live = BONE_CAGE_KEYS.map(|k| actor(&mut pruning, k, 120000));
    let live_phyl = actor(
        &mut pruning,
        b"oteryn:creature.bonelord_s_phylactery",
        50000,
    );
    let live_state = register(&pruning, live, live_phyl);
    pruning
        .register_bone_shared_hp(&live_state, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    for turn in 0..20 {
        let c = BONE_CAGE_KEYS.map(|k| actor(&mut pruning, k, 120000));
        let p = actor(
            &mut pruning,
            b"oteryn:creature.bonelord_s_phylactery",
            50000,
        );
        let state = register(&pruning, c, p);
        pruning
            .register_bone_shared_hp(&state, &fence, stamp)
            .expect("valid native fixture operation must succeed");
        assert_eq!(pruning.carrier.bone_shared_groups.len(), 2);
        assert!(
            pruning
                .carrier
                .bone_shared_groups
                .iter()
                .any(|g| g.cages == live)
        );
        for a in c.into_iter().chain(std::iter::once(p)) {
            pruning
                .carrier
                .remove(&pruning.continuity, a.0)
                .expect("valid native fixture operation must succeed");
        }
        assert_eq!(hp(&pruning, live[0]), 120000);
        assert!(turn < 20);
    }
    // Remove four cages, retaining only the exact HP0/present or live Phyl member still holds group.
    for a in live {
        pruning
            .carrier
            .remove(&pruning.continuity, a.0)
            .expect("valid native fixture operation must succeed");
    }
    let c = BONE_CAGE_KEYS.map(|k| actor(&mut pruning, k, 120000));
    let p = actor(
        &mut pruning,
        b"oteryn:creature.bonelord_s_phylactery",
        50000,
    );
    let state = register(&pruning, c, p);
    pruning
        .register_bone_shared_hp(&state, &fence, stamp)
        .expect("valid native fixture operation must succeed");
    assert!(
        pruning
            .carrier
            .bone_shared_groups
            .iter()
            .any(|g| g.phylactery == live_phyl)
    );
    assert_eq!(pruning.carrier.bone_shared_groups.len(), 2);
    println!(
        "PASS actual5 source roster/bindings; native generic4slot shareddamage/heal coalescence; fullpin/map/fence guards; replay/conflict/late-fourth atomicity; real4sealed cage keys openPhyl only after4; distinct finaldeath; no reward mint; stale replacement untouched. PROJECT120000, no spirit/map Global claims."
    );
}

#[cfg(any(test, feature = "test-support"))]
impl ChannelRuntimeV1 {
    pub fn bone_shared_fixture_damage(
        &mut self,
        actor: ExactActorRef,
        key: &str,
        damage: i64,
        occurrence: &str,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let binding = format!("{occurrence}\0{key}\0{damage}");
        self.borrow_exact_actor_commit().commit_damage(
            actor,
            OwnerDamageCommand {
                target: key.as_bytes(),
                occurrence: occurrence.as_bytes(),
                binding: binding.as_bytes(),
                damage,
            },
        )
    }
}
impl ChannelRuntimeV1 {
    /// Native occupancy veto under the SAME owner turn. Conservatively include all positioned
    /// actors (even dead/uncontrolled or differently pinned); never infer emptiness from timers.
    pub fn current_cell_occupied_except(
        &self,
        cell: MovementLocalPosition,
        except: ExactActorRef,
    ) -> bool {
        let Ok(excluded) = self.carrier.validate_ref(&self.continuity, except.0) else {
            return true;
        };
        self.carrier.slots.iter().enumerate().any(|(i, slot)| {
            if i == excluded {
                return false;
            }
            let position = match slot {
                Slot::Occupied {
                    position: Some(p), ..
                }
                | Slot::CreatureOccupied {
                    position: Some(p), ..
                } => p.position,
                _ => return false,
            };
            position.x == cell.x && position.y == cell.y && position.floor == cell.floor
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
mod selfheal_generation_guard_tests {
    use super::*;
    #[test]
    fn stale_generation_cannot_heal_replacement_or_prune_replay() {
        let id = |t: u8| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
        let w = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
        let mut r = ChannelRuntimeV1::from_committed_assignment(
            w,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::test(w),
        )
        .expect("valid native fixture operation must succeed");
        let actor = ExactActorRef(
            r.carrier
                .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.boreth", 100)
                .expect("valid native fixture operation must succeed"),
        );
        let reg = r
            .bind_creature_self_heal(
                "oteryn:creature.boreth",
                "oteryn:ability.creature.boreth.fixture_heal",
                1400,
                100,
                100,
            )
            .expect("valid native fixture operation must succeed");
        let mut ledger = CreatureSelfHealLedger::default();
        r.commit_creature_self_heal(&mut ledger, &reg, actor, 0, 100)
            .expect("valid native fixture operation must succeed");
        let index = r
            .carrier
            .validate_ref(&r.continuity, actor.0)
            .expect("valid native fixture operation must succeed");
        // Same occupied slot and same identity, different generation: stale token must refuse.
        if let Slot::CreatureOccupied { generation, .. } = &mut r.carrier.slots[index] {
            *generation += 1
        } else {
            assert!(
                matches!(r.carrier.slots[index], Slot::CreatureOccupied { .. }),
                "fixture must contain a native creature"
            )
        }
        assert_eq!(
            r.commit_creature_self_heal(&mut ledger, &reg, actor, 1, 100)
                .expect_err("fixture must reject this invalid operation"),
            CarrierError::StaleActorGeneration
        );
        assert!(matches!(
            r.carrier.slots[index],
            Slot::CreatureOccupied { health: 200, .. }
        ));
        assert_eq!(ledger.entries.len(), 1);
        assert_eq!(
            ledger
                .entries
                .get(actor, "oteryn:ability.creature.boreth.fixture_heal")
                .map(|entry| entry.sequence),
            Some(0)
        );
    }
}

#[path = "runtime_actor_icicle.rs"]
mod runtime_actor_icicle;

type NativeSourceHealProjection = (Vec<(ExactActorRef, String, u64, u64)>, Vec<usize>);
/// Immutable source-qualified admission description. It grants no scope or placement authority.
/// The source adapter binds exact child revision; current physical owner reads its active policy.
#[derive(Debug, Clone)]
pub struct NativeEncounterSpawnSpec {
    parent: Box<str>,
    child: Box<str>,
    child_revision: Box<str>,
    content: [u8; 32],
}
impl NativeEncounterSpawnSpec {
    pub fn qualified(
        parent: &str,
        child: &str,
        child_revision: &str,
        content: [u8; 32],
    ) -> Result<Self, CarrierError> {
        if content == [0; 32] {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        Ok(Self {
            parent: copy_native_summon_key(parent)?,
            child: copy_native_summon_key(child)?,
            child_revision: copy_native_summon_key(child_revision)?,
            content,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterSpawnError {
    StaleOwner,
    MissingMap,
    RefusedMap,
    Native(CarrierError),
}
impl ChannelRuntimeV1 {
    /// Immediate masterless source spawn, through the same physical admission/position/policy
    /// owner as native creatures. No static respawn source, master, intrinsic link or reward override.
    /// Current map proof is independently resolved under this same exclusive owner turn.
    pub fn admit_native_encounter_masterless(
        &mut self,
        current: &super::ScopeRuntimeFence,
        stamp: super::RuntimeWorkStamp,
        parent: ExactActorRef,
        spec: &NativeEncounterSpawnSpec,
        at: MovementLocalPosition,
        map_admits: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> Option<bool>,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        use EncounterSpawnError as E;
        let b = self.binding();
        if !current.is_current_for_scope(
            super::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !current.accepts_stamp(stamp)
        {
            return Err(E::StaleOwner);
        }
        if spec.content != self.content.server_artifact_digest
            || !self.matches_live_creature_identity(parent, spec.parent.as_bytes())
        {
            return Err(E::Native(CarrierError::CreatureTargetMismatch));
        }
        // This reads the active loader-installed qualified table. A matching display name
        // cannot substitute for the exact child definition or revision.
        let policy = self.companion_policy(&spec.child).map_err(E::Native)?;
        if policy.definition_key.as_str() != spec.child.as_ref()
            || policy.definition_revision.as_str() != spec.child_revision.as_ref()
        {
            return Err(E::Native(CarrierError::CreatureTargetMismatch));
        }
        match map_admits(self, at) {
            None => return Err(E::MissingMap),
            Some(false) => return Err(E::RefusedMap),
            Some(true) => {}
        }
        // `CREATUREAI0-RL-04` (Codex 4242954089), as for native summons: it replaces the D57
        // envelope of 64 live creatures.
        if self.carrier.occupied.len() >= CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
            || !self.native_summon_cell_free(at)
        {
            return Err(E::Native(CarrierError::CapacityExceeded));
        }
        let context = self.pinned_position_context();
        self.carrier
            .validate_position_context(context)
            .map_err(E::Native)?;
        // Existing carrier compare-commit behavior: either all physical fields are installed,
        // or every modified slot/free-list field is restored before returning a rejection.
        let slots_before = self.carrier.slots.clone();
        let free_before = self.carrier.free_head;
        let occupied_before = self.carrier.occupied.clone();
        let result = (|| {
            let actor = self.carrier.admit_creature(
                &self.continuity,
                ActorState(0),
                &spec.child,
                policy.maximum_health,
            )?;
            self.carrier.initialize_position(
                &self.continuity,
                actor,
                context,
                LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            )?;
            let actor = ExactActorRef(actor);
            self.install_creature_policy(actor, &spec.child)?;
            Ok(actor)
        })();
        if result.is_err() {
            self.carrier.slots = slots_before;
            self.carrier.free_head = free_before;
            self.carrier.occupied = occupied_before;
        }
        result.map_err(E::Native)
    }
}
impl ChannelRuntimeV1 {
    /// Source callback administrative removal: no lethal mint, corpse, loot or XP handoff.
    /// Exact occupied generation + source policy are current; immutable source pins are not grants.
    pub fn remove_native_encounter_subject(
        &mut self,
        current: &super::ScopeRuntimeFence,
        stamp: super::RuntimeWorkStamp,
        actor: ExactActorRef,
        expected_key: &str,
        expected_content: [u8; 32],
    ) -> Result<(), CarrierError> {
        let b = self.binding();
        if !current.is_current_for_scope(
            super::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !current.accepts_stamp(stamp)
        {
            return Err(CarrierError::WrongScope);
        }
        if self.content_pin().server_artifact_digest() != expected_content
            || !self.matches_live_creature_identity(actor, expected_key.as_bytes())
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let index = self
            .carrier
            .validate_summon_actor(&self.continuity, actor)?;
        self.carrier.assert_slot_spell_unreserved(index)?;
        // Existing removal cascades intrinsic children. Preflight every fallible current-slot
        // check before entering it; no mutation can occur between checks in this owner turn.
        let mut count = 0;
        for link in self.carrier.native_summons.children(actor) {
            count += 1;
            if count > 16 {
                return Err(CarrierError::CapacityExceeded);
            }
            let child = self
                .carrier
                .validate_summon_actor(&self.continuity, link.child)?;
            self.carrier.assert_slot_spell_unreserved(child)?;
            if self.carrier.native_summons.has_children(link.child) {
                return Err(CarrierError::CreatureTargetMismatch);
            }
        }
        self.carrier.remove(&self.continuity, actor.0).map(|_| ())
    }
}
/// Private intrinsic-link provenance; it cannot authorize a current owner mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeSummonOrigin {
    Behavior {
        index: usize,
    },
    SourceCallback {
        ability: Box<str>,
        revision: Box<str>,
        occurrence: Box<str>,
    },
}
impl NativeSummonOrigin {
    fn try_owned(&self) -> Result<Self, CarrierError> {
        match self {
            Self::Behavior { index } => Ok(Self::Behavior { index: *index }),
            Self::SourceCallback {
                ability,
                revision,
                occurrence,
            } => Ok(Self::SourceCallback {
                ability: copy_native_summon_key(ability)?,
                revision: copy_native_summon_key(revision)?,
                occurrence: copy_native_summon_key(occurrence)?,
            }),
        }
    }
}
impl NativeSummonAdmissionSpec {
    #[allow(
        clippy::too_many_arguments,
        reason = "Owner ABI keeps independently qualified source, current fence, exact actor and commit facts explicit"
    )]
    pub fn qualified_callback(
        parent: &str,
        child: &str,
        health: i64,
        ability: &str,
        revision: &str,
        occurrence: &str,
        total: u32,
        count: u32,
        content: [u8; 32],
    ) -> Result<Self, CarrierError> {
        let mut spec = Self::qualified(parent, child, health, 0, total, count, content)?;
        spec.origin = NativeSummonOrigin::SourceCallback {
            ability: copy_native_summon_key(ability)?,
            revision: copy_native_summon_key(revision)?,
            occurrence: copy_native_summon_key(occurrence)?,
        };
        Ok(spec)
    }
}
impl ChannelRuntimeV1 {
    pub fn native_summon_origin(
        &self,
        actor: ExactActorRef,
    ) -> Result<Option<NativeSummonOrigin>, CarrierError> {
        self.carrier
            .validate_summon_actor(&self.continuity, actor)?;
        Ok(self
            .carrier
            .native_summons
            .child(actor)
            .map(|l| l.origin.clone()))
    }
}
#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
mod native_callback_origin_tests {
    use super::*;
    #[test]
    fn source_callback_origin_is_not_behavior_index_and_current_fence_controls_admission() {
        // Foundation is path-loaded by PostgreSQL integration targets; use its own
        // actual carrier fixture, without depending on the unrelated transport test tree.
        let mut r = super::tests::runtime(8);
        let parent = r
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                "oteryn:creature.gaz_haragoth",
                10000,
            )
            .expect("valid native fixture operation must succeed");
        let b = r.binding();
        let (f, stamp) = super::super::crystal_timer_fixture(
            super::super::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("valid native fixture operation must succeed");
        let spec = NativeSummonAdmissionSpec::qualified_callback(
            "oteryn:creature.gaz_haragoth",
            "oteryn:creature.minion_of_gaz_haragoth",
            1000,
            "oteryn:ability.spell.gaz_haragoth_summon",
            "definition-r1",
            "ai-profile:source-fixture:0:attack:11",
            7,
            7,
            r.content_pin().server_artifact_digest(),
        )
        .expect("valid native fixture operation must succeed");
        let child = r
            .admit_native_summon(
                &f,
                stamp,
                parent,
                &spec,
                MovementLocalPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
            )
            .expect("valid native fixture operation must succeed");
        assert!(
            matches!(r.native_summon_origin(child).expect("valid native fixture operation must succeed"),Some(NativeSummonOrigin::SourceCallback{ability,occurrence,..})if ability.as_ref()=="oteryn:ability.spell.gaz_haragoth_summon"&&occurrence.as_ref()=="ai-profile:source-fixture:0:attack:11")
        );
        let (wrong, wrong_stamp) = super::super::crystal_timer_fixture(
            super::super::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            super::super::ScopeOwnershipGeneration::new(2)
                .expect("valid native fixture operation must succeed"),
        )
        .expect("valid native fixture operation must succeed");
        assert_eq!(
            r.admit_native_summon(
                &wrong,
                wrong_stamp,
                parent,
                &spec,
                MovementLocalPosition {
                    x: 102,
                    y: 100,
                    floor: 7
                }
            ),
            Err(CarrierError::WrongScope)
        );
        r.remove_test_actor(parent)
            .expect("valid native fixture operation must succeed");
        assert!(r.native_summon_origin(child).is_err());
    }

    /// `SPAWNCEIL-RL-02` (SPAWN-CEILINGS-1, Codex 4243039217): one retirement call examines at
    /// most 256 links, from where the previous one stopped, so it never costs every link of 640;
    /// the dead children it did not reach retire on later calls, and an admission first retires
    /// its own parent's dead children, so the deferred ones never refuse a summon.
    #[test]
    fn summon_retirement_is_bounded_per_call_and_an_admission_retires_its_parent_first() {
        const SUMMONERS: usize = 40;
        const CHILDREN: usize = 16;
        const LINKS: usize = SUMMONERS * CHILDREN;
        const BUDGET: usize = NATIVE_SUMMON_RETIREMENT_LINKS_PER_CALL_MAX;
        let mut r = super::tests::runtime(SUMMONERS * (CHILDREN + 1) + 8);
        let b = r.binding();
        let (f, stamp) = super::super::crystal_timer_fixture(
            super::super::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("the fixture fence is current");
        let spec = NativeSummonAdmissionSpec::qualified(
            "oteryn:creature.gaz_haragoth",
            "oteryn:creature.minion_of_gaz_haragoth",
            1000,
            0,
            16,
            16,
            r.content_pin().server_artifact_digest(),
        )
        .expect("the summon spec qualifies");
        let at = |i: usize, dx: usize, dy: usize| MovementLocalPosition {
            x: 100 + i32::try_from(6 * (i % 20) + dx).expect("a small column"),
            y: 100 + i32::try_from(6 * (i / 20) + dy).expect("a small row"),
            floor: 7,
        };
        let mut parents = Vec::new();
        let mut children = Vec::new();
        for i in 0..SUMMONERS {
            let parent = r
                .admit_monster_lab_creature(at(i, 0, 0), "oteryn:creature.gaz_haragoth", 10000)
                .expect("a summoner is admitted");
            for k in 0..CHILDREN {
                children.push((
                    i,
                    r.admit_native_summon(&f, stamp, parent, &spec, at(i, 1 + k % 4, 1 + k / 4))
                        .expect("every summoner summons 16 children"),
                ));
            }
            parents.push(parent);
        }
        assert_eq!(r.carrier.native_summons.len(), LINKS);
        // A pass over live links examines 256 of them and retires none.
        r.carrier.native_summons.retire_cursor = 0;
        assert_eq!(r.retire_native_summons(&f, stamp), Ok(0));
        assert_eq!(r.carrier.native_summons.retire_cursor, 256);
        for &(_, child) in &children {
            r.commit_monster_lab_damage(child, b"native-child-lethal", 1000)
                .expect("a child dies");
        }
        // 640 dead children: one call retires 256 of them (ordinals 256..512, summoners 16..31).
        assert_eq!(r.retire_native_summons(&f, stamp), Ok(BUDGET));
        assert_eq!(r.carrier.native_summons.len(), LINKS - BUDGET);
        assert_eq!(r.carrier.native_summons.retire_cursor, 512);
        // Summoner 15's dead children (ordinals 240..256) lie past the next pass (ordinals
        // 512..640, then wrapping, 0..128): its admission retires that pass and then them.
        const LAST: usize = 15;
        let last = parents[LAST];
        let summoned = r
            .admit_native_summon(&f, stamp, last, &spec, at(LAST, 1, 1))
            .expect("deferred dead children never refuse a summon");
        assert_eq!(r.native_summon_role(summoned), Ok(Some(last)));
        assert_eq!(r.native_summon_count(last, None), 1);
        assert_eq!(
            r.carrier.native_summons.len(),
            LINKS - 2 * BUDGET - CHILDREN + 1
        );
        assert_eq!(r.carrier.native_summons.retire_cursor, 128);
        for &(i, child) in &children {
            if i == LAST {
                assert!(r.native_summon_role(child).is_err());
            }
        }
        // The remaining dead children (summoners 8..14) retire on the next call; then nothing.
        let remaining = children
            .iter()
            .filter(|(_, child)| r.native_summon_role(*child).is_ok())
            .count();
        assert_eq!(remaining, 7 * CHILDREN);
        assert_eq!(r.retire_native_summons(&f, stamp), Ok(remaining));
        assert_eq!(r.retire_native_summons(&f, stamp), Ok(0));
        assert_eq!(r.carrier.native_summons.len(), 1);
        for &parent in &parents {
            assert_eq!(
                r.native_summon_count(parent, None),
                u32::from(parent == last)
            );
        }
    }

    /// SPAWN-CEILINGS-1 (Codex 4243039220): the memo table answers by exact creature and entry
    /// past the D57 envelope; a creature's first touch drops only what its slot's former
    /// occupant left, and the full-table prune drops only dead creatures' memos.
    #[test]
    fn actor_memo_table_is_keyed_by_exact_creature_and_drops_stale_memos_by_slot() {
        const CREATURES: usize = 300;
        const ENTRIES: usize = CREATUREAI0_SUMMON_ENTRIES_PER_CREATURE_MAX;
        let mut r = super::tests::runtime(CREATURES + 8);
        let mut table = ActorMemoTable::<usize, u64>::default();
        assert!(table.is_empty());
        let mut actors = Vec::new();
        for i in 0..CREATURES {
            let actor = ExactActorRef(
                r.carrier
                    .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.boreth", 1000)
                    .expect("a creature is admitted"),
            );
            for entry in 0..ENTRIES {
                assert_eq!(
                    table.insert(actor, entry, (i * ENTRIES + entry) as u64),
                    None
                );
            }
            actors.push(actor);
        }
        assert_eq!(table.len(), CREATURES * ENTRIES);
        for (i, &actor) in actors.iter().enumerate() {
            assert_eq!(table.actor_len(actor), ENTRIES);
            assert_eq!(table.get(actor, &3), Some(&((i * ENTRIES + 3) as u64)));
        }
        assert_eq!(
            table.insert(actors[7], 3, 1),
            Some((7 * ENTRIES + 3) as u64)
        );
        assert_eq!(table.len(), CREATURES * ENTRIES);
        // The first creature's slot is taken again: its memos never answer the new occupant,
        // and the new occupant's first touch drops exactly them.
        r.remove_test_actor(actors[0]).expect("removed");
        let next = ExactActorRef(
            r.carrier
                .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.boreth", 1000)
                .expect("a creature is admitted"),
        );
        assert_eq!(next.actor_local_id(), actors[0].actor_local_id());
        assert_eq!(table.get(next, &3), None);
        assert_eq!(table.actor_len(next), 0);
        assert_eq!(table.evict_stale_at(actors[1]), 0);
        assert_eq!(table.evict_stale_at(next), ENTRIES);
        assert_eq!(table.len(), (CREATURES - 1) * ENTRIES);
        assert_eq!(table.get(actors[0], &3), None);
        // A dead creature whose slot nobody reuses keeps its memos until the full-table prune.
        r.remove_test_actor(actors[1]).expect("removed");
        assert_eq!(table.insert(next, 0, 9), None);
        assert_eq!(table.len(), (CREATURES - 1) * ENTRIES + 1);
        table.prune(|actor| r.contains_live_creature(actor));
        assert_eq!(table.len(), (CREATURES - 2) * ENTRIES + 1);
        assert_eq!(table.actor_len(actors[1]), 0);
        assert_eq!(table.actor_len(actors[2]), ENTRIES);
        assert_eq!(table.get(next, &0), Some(&9));
        assert!(!table.is_empty());
        table.prune(|_| false);
        assert!(table.is_empty());
        assert_eq!(table.get(next, &0), None);
    }
}

/// Project the already committed lethal transition. Combat cannot manufacture
/// lethality, actor identity or position: it can only ask the physical owner
/// for an opaque receipt and immediately return that receipt for projection.
pub fn project_fixed_one_creature_death<'a>(
    owner: &'a mut CurrentOwnerCombatDeath<'_>,
    actor: ExactActorRef,
) -> Result<&'a RuntimeCorpseProjection, CarrierError> {
    let receipt = owner.committed_lethal_receipt(actor)?;
    owner.project_committed_lethal(receipt)
}

#[cfg(any(test, feature = "test-support"))]
impl crate::foundation::CombatDeathFixture {
    /// Combat's idempotent projection of the committed lethal occurrence and
    /// the durable death key it names, with the corpse position.
    pub fn project_death(
        &mut self,
    ) -> Result<
        (
            crate::foundation::CreatureDeathOccurrenceKey,
            crate::foundation::MovementLocalPosition,
        ),
        CarrierError,
    > {
        let actor = self.actor();
        let mut combat = self.borrow_combat_death();
        let projection = project_fixed_one_creature_death(&mut combat, actor)?;
        Ok((projection.occurrence().death_key(), projection.position()))
    }
}
