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
use std::mem::size_of;
use std::sync::Arc;

#[path = "runtime_actor_conditions.rs"]
mod runtime_actor_conditions;
#[allow(unused_imports)] // The Foundation facade is composed by the owning consumer child.
pub(crate) use runtime_actor_conditions::{
    ActorConditionPlan, ActorConditionTransition, ApplicationFacts, ConditionDefinition,
    ConditionOwnerError, ConditionSource, ConditionSourceKind, ConditionStore, ConditionType,
    ConditionValues, SpeedRange,
};
use runtime_actor_conditions::{CreatureCommitState, PlayerRuntimeState};
#[path = "runtime_actor_death.rs"]
mod runtime_actor_death;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarrierError {
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
    /// the registered AI01-SPAWN-* bound.
    InvalidSpawnDefinition,
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
struct PreProductionContinuityGrant {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_generation: ScopeOwnershipGeneration,
}

/// Surviving pre-production namespace continuity state.
///
/// This guard deliberately lives outside carrier backing and is neither
/// `Clone` nor `Copy`. A carrier claims the current generation once; losing
/// that carrier therefore cannot make the generation claimable again.
#[derive(Debug)]
struct NamespaceContinuityGuard {
    world_id: WorldId,
    channel_id: ChannelId,
    current_generation: ScopeOwnershipGeneration,
    current_generation_claimed: bool,
}

impl NamespaceContinuityGuard {
    fn from_pre_production_grant(grant: PreProductionContinuityGrant) -> Self {
        Self {
            world_id: grant.world_id,
            channel_id: grant.channel_id,
            current_generation: grant.scope_generation,
            current_generation_claimed: false,
        }
    }

    fn advance(&mut self, grant: PreProductionContinuityGrant) -> Result<(), CarrierError> {
        if grant.world_id != self.world_id || grant.channel_id != self.channel_id {
            return Err(CarrierError::WrongScope);
        }
        if grant.scope_generation.get() <= self.current_generation.get() {
            return Err(CarrierError::ContinuityGenerationNotNewer);
        }

        self.current_generation = grant.scope_generation;
        self.current_generation_claimed = false;
        Ok(())
    }

    fn claim_current_generation(&mut self) -> Result<(), CarrierError> {
        self.ensure_current_generation_unclaimed()?;
        self.current_generation_claimed = true;
        Ok(())
    }

    fn ensure_current_generation_unclaimed(&self) -> Result<(), CarrierError> {
        if self.current_generation_claimed {
            return Err(CarrierError::NamespaceAlreadyClaimed);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorLocalId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorLocalGeneration(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorRef {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_generation: ScopeOwnershipGeneration,
    actor_local_id: ActorLocalId,
    actor_local_generation: ActorLocalGeneration,
}

/// Inseparable semantic actor handle issued by this one Channel carrier.
/// Its fields and constructor are private to Foundation; it is never decoded
/// from a client handle or derived from the fixture Ability `TargetId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactActorRef(ActorRef);

impl ExactActorRef {
    /// Stable opaque placement identity: a domain-separated digest of the exact actor
    /// reference in its runtime scope. It carries no position and reveals no slot layout.
    pub(crate) fn placement_identity(self) -> [u8; 16] {
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
    #[cfg(test)]
    #[allow(clippy::expect_used, dead_code)]
    pub(crate) fn transport_fixture(world_id: WorldId, channel_id: ChannelId) -> Self {
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
pub(crate) struct PlayerActorReservation {
    game_session_id: GameSessionId,
    actor_ref: ActorRef,
}

/// Immutable provenance of the committed Channel assignment that created this
/// runtime. Source revision identifies the exact assignment decision; no
/// production capacity value lives here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChannelRuntimeAssignmentBinding {
    world_id: WorldId,
    channel_id: ChannelId,
    node_id: NodeId,
    node_registration_revision: u64,
    scope_generation: ScopeOwnershipGeneration,
    source_revision: u64,
}

impl ChannelRuntimeAssignmentBinding {
    pub(crate) const fn world_id(self) -> WorldId {
        self.world_id
    }

    pub(crate) const fn channel_id(self) -> ChannelId {
        self.channel_id
    }

    pub(crate) const fn scope_generation(self) -> ScopeOwnershipGeneration {
        self.scope_generation
    }

    /// The committed assignment decision revision this runtime was composed from.
    pub(crate) const fn source_revision(self) -> u64 {
        self.source_revision
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn matches_committed_assignment(
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
pub(crate) struct CurrentOwnerExactActorLookup<'a> {
    carrier: &'a ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
}

/// A short-lived mutable owner borrow. No resolved snapshot can grant this
/// capability: the carrier checks independent continuity and slot generation
/// again at the sole write.
pub(crate) struct CurrentOwnerExactActorCommit<'a> {
    carrier: &'a mut ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
}

/// Short-lived current-owner authority for the fixed one-creature death
/// projection. It cannot commit Ability damage, move actors, admit actors or
/// issue continuity.
pub(crate) struct CurrentOwnerCombatDeath<'a> {
    carrier: &'a mut ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
}

/// Borrowed, current-owner position capability for one ordinary actor slot.
/// It cannot admit/remove an actor, issue continuity, or commit Ability damage.
pub(crate) struct CurrentOwnerMovementPosition<'a> {
    carrier: &'a mut ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MovementLocalPosition {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) floor: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MovementPositionContext(PreProductionPositionContext);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MovementPositionSnapshot(PositionSnapshot);

impl MovementPositionSnapshot {
    pub(crate) const fn position(self) -> MovementLocalPosition {
        let position = self.0.version.position;
        MovementLocalPosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        }
    }

    pub(crate) const fn context(self) -> MovementPositionContext {
        MovementPositionContext(self.0.version.context)
    }

    pub(crate) const fn revision(self) -> u64 {
        self.0.version.revision
    }

    pub(crate) const fn world_id(self) -> WorldId {
        self.0.version.context.world_id
    }
}

impl CurrentOwnerMovementPosition<'_> {
    pub(crate) fn read(
        &self,
        actor: ExactActorRef,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .read_movement_position(self.continuity, actor.0)
    }

    pub(crate) fn commit_cardinal(
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
pub(crate) struct OwnerDamageResult {
    pub(crate) applied: bool,
    pub(crate) health_before: i64,
    pub(crate) health_after: i64,
}

/// Immutable, borrowed Ability proposal. Authority is checked by the owner,
/// independently of these bytes, before the single slot replacement.
pub(crate) struct OwnerDamageCommand<'a> {
    pub(crate) target: &'a [u8],
    pub(crate) occurrence: &'a [u8],
    pub(crate) binding: &'a [u8],
    pub(crate) damage: i64,
}

/// `COMBAT01-DAMAGE-RECEIPTS-PER-CREATURE-GENERATION` (`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
/// D140, `docs/architecture/reviews/OTERYN_GAME_D4_MULTI_HIT_DAMAGE_RECEIPT_DECISION_2026-09-29.md`
/// section 4.1): at most this many committed damage receipts are retained at once per live
/// creature actor generation. Retention, not admission: an evictable oldest receipt is evicted to
/// admit a new distinct occurrence (D141); only 16 simultaneously non-evictable receipts refuse
/// one with [`CarrierError::DamageReceiptCapacityExceeded`].
pub(crate) const COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX: usize = 16;

/// Mirror of the already-registered `ABILITY01-EFFECT-PLAN-ENTRIES` hard maximum
/// (`ability/mod.rs` `MAX_EFFECT_PLAN_ENTRIES`, no new row): the number of distinct
/// `sub_ordinal` values one command can produce. Foundation must not import Ability, so the
/// value is mirrored here and pinned to it by a test in `channel_owner_ability_commit_tests.rs`.
pub(crate) const ABILITY01_EFFECT_PLAN_ENTRIES_MAX: u16 = 2;

/// `(CharacterId, character_lease_generation, GameSessionId, CommandId sequence, sub_ordinal)`:
/// the carrier-derived replay identity of one attributed damage occurrence (D141 point 3).
type DamageOrigin = (CharacterId, u64, GameSessionId, u64, u16);

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
pub(crate) struct AttackerCommand {
    character: CharacterId,
    lease_generation: u64,
    session: GameSessionId,
    sequence: u64,
    sub_ordinal: u16,
}

impl AttackerCommand {
    /// `lease_generation` is the attacker's current `character_lease_generation` (non-zero).
    pub(crate) const fn new(
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

    const fn origin(self) -> DamageOrigin {
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
struct OwnerCommitRecord {
    binding: Box<[u8]>,
    damage: i64,
    result: OwnerDamageResult,
    /// D142: this creature generation's owner damage-application ordinal of this receipt.
    ordinal: u64,
    /// D141: `Some` for an attributed commit (its replay identity), `None` for an unsequenced
    /// one, which is never evictable.
    origin: Option<DamageOrigin>,
}

/// D140/D142: the bounded, ephemeral, per-creature-generation receipt list and the owner
/// damage-application ordinal counter. Fresh on every admission, dropped with the slot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DamageReceipts {
    entries: Vec<OwnerCommitRecord>,
    next_ordinal: u64,
}

impl DamageReceipts {
    /// D144: the one retained receipt whose commit drove health to zero, if any.
    fn lethal(&self) -> Option<&OwnerCommitRecord> {
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
pub(crate) const COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX: usize = 16;

/// D132: one tracked attacker's running damage total against a live creature actor, and the
/// ordinal (this creature's own damage-application sequence) at which that total was last
/// reached. A contributor's total only ever increases, so "last reached" already is "first
/// reached this exact value" -- no separate history is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DamageContributor {
    character: CharacterId,
    total: u64,
    last_update_ordinal: u64,
    /// D141: this attacker's `(session, sequence, sub_ordinal)` high-water mark. A new current
    /// session replaces it outright; within one session it only ever rises.
    high_water: Option<HighWater>,
}

/// D132/D3-3: bounded, ephemeral, per-creature-generation damage-contributor accumulation.
/// Lives only inside its owning `Slot::CreatureOccupied` -- a fresh, empty map on every creature
/// admission (`admit_inner`), dropped with the slot on `remove` (administrative removal or
/// respawn). Never grows past `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX` and never
/// initialized from, or retained across, a prior generation: a scope move drops whatever
/// in-progress accumulation was pending, exactly the accepted D52 loss ("Przepadają, bez
/// duplikatów" -- nothing duplicated, nothing silently completed by a later generation).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DamageContributors {
    entries: Vec<DamageContributor>,
}

impl DamageContributors {
    /// Attribute one already-applied, positive damage amount to `character`. A *new* distinct
    /// contributor past `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX` is silently not tracked:
    /// the caller's own HP mutation already committed independently of this call and is never
    /// affected by it. An already-tracked contributor keeps accumulating regardless of map size.
    ///
    /// `ordinal` is the owner damage-application ordinal (D142) the carrier assigned to this
    /// commit at the same mutation boundary; this accumulator keeps no counter of its own.
    /// `high_water`, when supplied, replaces the attacker's mark (D141).
    fn record(
        &mut self,
        character: CharacterId,
        damage: u64,
        ordinal: u64,
        high_water: Option<HighWater>,
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
        self.entries
            .iter()
            .copied()
            .reduce(|best, candidate| {
                if is_more_preferred_contributor(&candidate, &best) {
                    candidate
                } else {
                    best
                }
            })
            .map(|contributor| contributor.character)
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

/// Stable identity of the one committed lethal occurrence. Construction stays
/// private to the physical Channel owner; callers cannot supply occurrence
/// bytes, HP facts or actor generation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CreatureDeathOccurrenceRef {
    actor: ExactActorRef,
    commit_binding: Box<[u8]>,
    damage: i64,
    health_before: i64,
}

impl CreatureDeathOccurrenceRef {
    pub(crate) const fn actor(&self) -> ExactActorRef {
        self.actor
    }

    pub(crate) fn commit_binding(&self) -> &[u8] {
        &self.commit_binding
    }

    pub(crate) const fn damage(&self) -> i64 {
        self.damage
    }

    pub(crate) const fn health_before(&self) -> i64 {
        self.health_before
    }

    /// The durable, restart-stable death key of this committed lethal
    /// occurrence (`CREATURE-DEATH-OCCURRENCE-IDENTITY-V1`, DUR-03 decision
    /// §4.1): exactly the owner-issued actor reference. The commit binding,
    /// damage and HP facts stay bound attributes, never key material.
    pub(crate) const fn death_key(&self) -> CreatureDeathOccurrenceKey {
        CreatureDeathOccurrenceKey(self.actor.0)
    }
}

/// `(WorldId, ChannelId, ScopeOwnershipGeneration, ActorLocalId,
/// ActorLocalGeneration)` of one committed creature death. The only
/// constructor is [`CreatureDeathOccurrenceRef::death_key`]: there is no
/// raw-byte, field or caller constructor, so a key always names a lethal
/// occurrence the physical Channel owner committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreatureDeathOccurrenceKey(ActorRef);

impl CreatureDeathOccurrenceKey {
    pub(crate) const fn world_id(self) -> WorldId {
        self.0.world_id
    }

    pub(crate) const fn channel_id(self) -> ChannelId {
        self.0.channel_id
    }

    pub(crate) const fn scope_ownership_generation(self) -> ScopeOwnershipGeneration {
        self.0.scope_generation
    }

    pub(crate) const fn actor_local_id(self) -> u32 {
        self.0.actor_local_id.0
    }

    pub(crate) const fn actor_local_generation(self) -> u64 {
        self.0.actor_local_generation.0
    }
}

/// One runtime-owned, non-persistent corpse projection. The position is the
/// immutable current-owner position captured by the same slot state that holds
/// the lethal commit; dead creatures cannot mutate or initialize it later.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RuntimeCorpseProjection {
    occurrence: CreatureDeathOccurrenceRef,
    position: VersionedPosition,
}

impl RuntimeCorpseProjection {
    pub(crate) const fn occurrence(&self) -> &CreatureDeathOccurrenceRef {
        &self.occurrence
    }

    pub(crate) const fn position(&self) -> MovementLocalPosition {
        MovementLocalPosition {
            x: self.position.position.x,
            y: self.position.position.y,
            floor: self.position.position.floor,
        }
    }

    pub(crate) const fn position_revision(&self) -> u64 {
        self.position.revision
    }

    pub(crate) const fn context_markers(&self) -> (u64, u64, u64) {
        (
            self.position.context.coordinate_frame_marker,
            self.position.context.map_revision_marker,
            self.position.context.content_generation_marker,
        )
    }
}

/// Opaque, single-use handoff from the owner commit record to Combat. It is
/// deliberately neither Clone nor Copy and has no caller-visible constructor.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CommittedLethalReceipt {
    projection: RuntimeCorpseProjection,
}

/// 4096 is the registered Ability plan bound; the owner still independently
/// checks the actual complete encoding before allocating its one receipt.
const MAX_OWNER_COMMIT_BINDING_BYTES: usize = 4_096;

impl CurrentOwnerExactActorCommit<'_> {
    pub(crate) fn commit_damage(
        &mut self,
        actor: ExactActorRef,
        command: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.carrier
            .commit_creature_damage_inner(self.continuity, actor.0, command, None, false)
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
    #[cfg(test)]
    pub(crate) fn commit_damage_for_attacker(
        &mut self,
        actor: ExactActorRef,
        attacker: AttackerCommand,
        command: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.carrier.commit_creature_damage_inner(
            self.continuity,
            actor.0,
            command,
            Some(attacker),
            false,
        )
    }
}

impl CurrentOwnerExactActorCommit<'_> {
    /// A2 (D295 item 4): the current lease of the attacker's bound player slot. The slot must hold
    /// the command's session, carry no write fence and have a bound lease, checked in that order
    /// (CHARM-DESC-FENCE-LEASE §3 item 2); anything else is `SupersededAttackerSession`.
    pub(crate) fn bound_attacker_lease(
        &self,
        attacker: ExactActorRef,
        command: CommandRef,
    ) -> Result<CharacterLease, CarrierError> {
        self.carrier
            .bound_attacker_lease(self.continuity, attacker.0, command)
    }

    /// The attributed damage commit whose attacker authority is the bound slot of `attacker`,
    /// read in the same runtime-lock critical section as the write.
    pub(crate) fn commit_damage_for_bound_attacker(
        &mut self,
        actor: ExactActorRef,
        attacker: ExactActorRef,
        command: CommandRef,
        sub_ordinal: u16,
        damage: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let lease = self.bound_attacker_lease(attacker, command)?;
        self.carrier.commit_creature_damage_inner(
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
        )
    }
}

impl CurrentOwnerCombatDeath<'_> {
    pub(crate) fn committed_lethal_receipt(
        &self,
        actor: ExactActorRef,
    ) -> Result<CommittedLethalReceipt, CarrierError> {
        self.carrier
            .committed_lethal_receipt_inner(self.continuity, actor.0)
    }

    pub(crate) fn project_committed_lethal(
        &mut self,
        receipt: CommittedLethalReceipt,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        self.carrier
            .project_committed_lethal_inner(self.continuity, receipt, false, false)
    }

    #[cfg(test)]
    pub(crate) fn project_committed_lethal_with_failures(
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
    pub(crate) fn projected_death(
        &self,
        actor: ExactActorRef,
    ) -> Result<(CreatureDeathOccurrenceKey, MovementLocalPosition), CarrierError> {
        self.carrier.validate_ref(self.continuity, actor.0)?;
        self.carrier
            .corpse_projections
            .iter()
            .find(|projection| projection.occurrence.actor == actor)
            .map(|projection| (projection.occurrence.death_key(), projection.position()))
            .ok_or(CarrierError::CommittedLethalUnavailable)
    }

    /// D2b: the memoized XP `ExperienceRewardOccurrence` bytes of this
    /// generation's committed death for `character`, minting them on first
    /// call. See [`ChannelActorCarrier::reward_occurrence_inner`].
    pub(crate) fn reward_occurrence(
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
    pub(crate) fn top_damage_character(
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
    pub(crate) fn contains(&self, actor: ExactActorRef) -> bool {
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
            Slot::VacantReusable { .. } | Slot::Exhausted { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActorState(u64);

/// The binding a position is valid under. In production the Channel's fixed
/// Content pin (#935) holds the full binding once; each slot keeps only this
/// compact per-runtime reference to it (the measured 192-byte slot footprint
/// is unchanged): the frame-binding and map-revision digest prefixes and the
/// activation sequence. Test fixtures use synthetic values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreProductionPositionContext {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_generation: ScopeOwnershipGeneration,
    coordinate_frame_marker: u64,
    map_revision_marker: u64,
    content_generation_marker: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalPosition {
    x: i32,
    y: i32,
    floor: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VersionedPosition {
    actor_local_id: ActorLocalId,
    actor_local_generation: ActorLocalGeneration,
    context: PreProductionPositionContext,
    position: LocalPosition,
    revision: u64,
}

/// A value snapshot, never an authority token. Compare-commit revalidates
/// actor, independently current owner, context and revision at the write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PositionSnapshot {
    actor_ref: ActorRef,
    version: VersionedPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Slot {
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
    },
    Exhausted {
        generation: u64,
    },
}

/// The durable loss decision mirrored by the Channel owner for its present, uncontrolled
/// player actor. Epoch and grace deadline are the committed `ControlLossEpoch` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ControlLossMark {
    pub(crate) epoch: u64,
    pub(crate) grace_deadline: i64,
}

/// CHARM-DESC-FENCE-LEASE §3 item 2: the token of one transition's write fence on a player slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteFenceToken {
    /// Grace expiry: the control-loss epoch the slot still carries.
    ControlLoss(u64),
    /// Any other transition that ends the session's hold: an id minted by
    /// [`ChannelRuntimeV1::mint_transition_fence`] for the caller.
    Transition(u64),
}

/// How [`ChannelRuntimeV1::fence_player_writes`] set the fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteFenceSet {
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
pub(crate) enum ContinuationReconcile {
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
pub(crate) struct PlayerControlFacts {
    /// Stable opaque identity of this actor's placement in this runtime scope.
    pub(crate) placement_identity: [u8; 16],
    /// The actor's local generation: non-zero and unchanged while it stays placed.
    pub(crate) placement_revision: u64,
    pub(crate) control_loss: Option<ControlLossMark>,
}

#[derive(Debug, PartialEq, Eq)]
struct ChannelActorCarrier {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_generation: ScopeOwnershipGeneration,
    slots: Box<[Slot]>,
    free_head: Option<u32>,
    /// AI-2 (GAME-AI-01 §4.1: "the carrier's current one-creature limit ...
    /// is lifted to this envelope in AI-2"): one committed death per creature
    /// generation, keyed by that creature's own `ExactActorRef`. Bounded by
    /// `slots.len()` itself -- no more than one dead-but-not-yet-removed
    /// creature can exist per slot, so this can never exceed total capacity.
    /// `remove` prunes an actor's entry so it never outlives its slot.
    corpse_projections: Vec<RuntimeCorpseProjection>,
    /// D2b: the memoized XP reward occurrence of each committed death, minted
    /// at most once per actor (DUR-03 decision §4.2). AI-2 widens this from
    /// its original one-creature-per-generation singleton to one entry per
    /// dead creature actor, same bound and pruning as `corpse_projections`. A
    /// fresh carrier is bootstrapped per ownership generation, so this is
    /// never initialized from a prior generation: a scope move drops whatever
    /// was pending here rather than reusing or duplicating it (D52).
    death_reward_occurrences: Vec<DeathRewardOccurrence>,
    /// AI-2 (§4.3, D116): realized spawn sources and their per-cell live/
    /// pending/retry state for this carrier's ownership generation.
    spawns: Vec<SpawnRealization>,
    /// The last write-fence transition id this carrier minted (CHARM-DESC-FENCE-LEASE §3 item 2).
    fence_transitions: u64,
    /// A2: the bound lease and write fence of each committed player slot that has one, keyed by
    /// slot index and that slot's generation. At most one entry per slot: `remove` prunes it, so
    /// it is bounded by `slots.len()` and never outlives its actor.
    attackers: Vec<PlayerAttackerEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlayerAttackerEntry {
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

// AI-2 (GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1 §4.3, §4.9, D115/D116):
// spawn realization and respawn. `RESOURCE_LIMITS_REGISTRY.json`'s AI01-SPAWN-* rows are the
// source of truth for these ceilings; the constants below must equal the registered values.

/// AI01-SPAWN-SOURCES-PER-SCOPE (AI-RL-11).
pub(crate) const AI01_SPAWN_SOURCES_PER_SCOPE_MAX: usize = 16;
/// AI01-SPAWN-POPULATION (AI-RL-11): live or pending creatures per source.
pub(crate) const AI01_SPAWN_POPULATION_MAX: usize = 4;
/// AI01-SPAWN-PLACEMENT-CELLS (AI-RL-12): declared cells per source, one per creature.
pub(crate) const AI01_SPAWN_PLACEMENT_CELLS_MAX: usize = 4;
/// AI01-SPAWN-OCCUPANCY-RETRIES (AI-RL-13): retries per creature per respawn window.
pub(crate) const AI01_SPAWN_OCCUPANCY_RETRIES_MAX: u8 = 3;

/// Bounded FIFO retention for `corpse_projections`/`death_reward_occurrences`, which
/// deliberately outlive their actor's removed slot (lost-response replay). Matches the D57
/// envelope's total creature count (`AI01_SPAWN_SOURCES_PER_SCOPE_MAX *
/// AI01_SPAWN_POPULATION_MAX`), generously beyond D116's 2-creature content: eviction is a
/// defensive bound against unbounded growth over a long-lived Channel's respawn cycles, not a
/// dimension this slice's tests are expected to reach.
const MAX_RETAINED_CORPSE_PROJECTIONS: usize =
    AI01_SPAWN_SOURCES_PER_SCOPE_MAX * AI01_SPAWN_POPULATION_MAX;

/// A content-authored spawn source's small stable ordinal within its scope (content input,
/// §4.8; never decoded from a client handle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SpawnSourceId(pub(crate) u16);

/// §4.3/§4.8: one spawn source's content-declared definition. Code never invents these
/// values; a missing one is a content-validation failure at a higher layer, not here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpawnDefinition {
    /// §4.3: "one placement cell per creature" -- exactly `population` cells, never more
    /// (`placement_cells.len() == population <= AI01_SPAWN_PLACEMENT_CELLS_MAX`). Placement
    /// never searches beyond these.
    placement_cells: Vec<LocalPosition>,
    /// `1..=AI01_SPAWN_POPULATION_MAX`, and exactly `placement_cells.len()`.
    population: usize,
    /// D115: 60,000,000 (60 s, in the owner clock's microsecond unit).
    respawn_delay_micros: u64,
    /// D115: 5,000,000 (5 s).
    occupancy_retry_interval_micros: u64,
    creature_target_identity: String,
    creature_initial_health: i64,
}

impl SpawnDefinition {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        placement_cells: Vec<(i32, i32, i16)>,
        population: usize,
        respawn_delay_micros: u64,
        occupancy_retry_interval_micros: u64,
        creature_target_identity: String,
        creature_initial_health: i64,
    ) -> Result<Self, CarrierError> {
        if placement_cells.is_empty() || placement_cells.len() > AI01_SPAWN_PLACEMENT_CELLS_MAX {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        // §4.3: "one declared cell per creature" -- a mismatch either way (fewer cells than
        // the population, or extra unused cells) is invalid, not merely clamped, so
        // `resolve_respawn_timer` can never respawn more than the declared population.
        if population == 0
            || population != placement_cells.len()
            || population > AI01_SPAWN_POPULATION_MAX
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        if creature_initial_health <= 0 {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        Ok(Self {
            placement_cells: placement_cells
                .into_iter()
                .map(|(x, y, floor)| LocalPosition { x, y, floor })
                .collect(),
            population,
            respawn_delay_micros,
            occupancy_retry_interval_micros,
            creature_target_identity,
            creature_initial_health,
        })
    }
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
pub(crate) const AI01_PENDING_RESPAWN_TIMERS_PER_DEAD_ACTOR: usize = 1;

/// AI-2's one timer family: respawn (§4.2's table: `DEADLINE_STATE`, distinct from AI-1's
/// `SKIP_TO_LATEST` think family -- every due retry/successor must still fire, never collapsed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RespawnFamily {
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
pub(crate) struct RespawnOccurrence {
    pub(crate) source: SpawnSourceId,
    pub(crate) cell_index: u8,
    pub(crate) dead_actor: ExactActorRef,
    pub(crate) successor: u32,
    pub(crate) attempt: u8,
}

/// The result of applying one fired respawn timer (§4.3). This never mutates state itself
/// beyond `ChannelActorCarrier::resolve_respawn_timer`'s own admission/bookkeeping; scheduling
/// the next timer (if any) is the caller's job, exactly as `OwnerTimerLane` requires (binding
/// item (b): the caller's own fence-issued `RuntimeWorkStamp`, never fabricated here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RespawnResolution {
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
pub(crate) struct ChannelContentPin {
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
    pub(crate) const fn from_activation(
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
    #[cfg(test)]
    pub(crate) const fn test(world_id: WorldId) -> Self {
        Self::from_activation(world_id, 1, [1; 32], [2; 32], [3; 32], [4; 32], (0, 0, 0))
    }

    pub(crate) const fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub(crate) const fn activation_sequence(&self) -> u64 {
        self.activation_sequence
    }

    pub(crate) const fn server_artifact_digest(&self) -> [u8; 32] {
        self.server_artifact_digest
    }

    pub(crate) const fn client_artifact_digest(&self) -> [u8; 32] {
        self.client_artifact_digest
    }

    pub(crate) const fn frame_binding_digest(&self) -> [u8; 32] {
        self.frame_binding_digest
    }
}

/// Outcome of a first-entry position initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FirstEntryPosition {
    /// This call wrote the start position.
    Initialized(MovementPositionSnapshot),
    /// The same initialization had already completed; nothing was written.
    Reconciled(MovementPositionSnapshot),
}

/// The first composed Channel runtime. The fixed-slot carrier is the only actor
/// resource: no session map or second index is introduced.
#[derive(Debug)]
pub(crate) struct ChannelRuntimeV1 {
    binding: ChannelRuntimeAssignmentBinding,
    continuity: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    content: ChannelContentPin,
}

impl ChannelRuntimeV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_committed_assignment(
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
        })
    }

    pub(crate) const fn binding(&self) -> ChannelRuntimeAssignmentBinding {
        self.binding
    }

    pub(crate) const fn content_pin(&self) -> &ChannelContentPin {
        &self.content
    }

    /// Unactivated Movement proof: one exclusive borrow of the composed Channel owner.
    /// This grants neither initial position authority nor an owner scheduler.
    pub(crate) fn borrow_movement_position(&mut self) -> CurrentOwnerMovementPosition<'_> {
        self.carrier
            .current_owner_movement_position(&self.continuity)
    }

    /// The Movement owner context of this runtime's fixed Content pin; a step is eligible only
    /// for a position initialized under exactly this context.
    pub(crate) fn pinned_movement_context(&self) -> MovementPositionContext {
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
    pub(crate) fn committed_player_positions(&self) -> Vec<MovementLocalPosition> {
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

    /// AI-4 (GAME-AI-01 slice §4.6): true only while `actor` names a live (`health > 0`)
    /// creature generation of this runtime. A player, a stale or dead generation, a vacant slot
    /// or another scope is false.
    pub(crate) fn contains_live_creature(&self, actor: ExactActorRef) -> bool {
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
    pub(crate) fn read_actor_position(
        &self,
        actor: ExactActorRef,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .read_position(&self.continuity, actor.0)
            .map(MovementPositionSnapshot)
    }

    /// Unactivated Combat proof: one exclusive borrow of the physical Channel
    /// owner. It grants no scheduler, production activation or corpse lifetime.
    pub(crate) fn borrow_combat_death(&mut self) -> CurrentOwnerCombatDeath<'_> {
        self.carrier.current_owner_combat_death(&self.continuity)
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
    pub(crate) fn initialize_first_entry_position(
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
    pub(crate) fn player_control_facts(
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

    /// Mirror one committed durable control loss onto the still-present player actor.
    /// Recording the identical decision again is a no-op; a different one conflicts.
    pub(crate) fn record_control_loss(
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
    pub(crate) fn restore_control(
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
    #[cfg(test)]
    pub(crate) fn initialize_movement_test_position(
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

    #[cfg(test)]
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

    /// Test only: one live creature at `position`, under the same synthetic context as
    /// [`Self::initialize_movement_test_position`]. Spawn realization is AI-2's carrier path.
    #[cfg(test)]
    pub(crate) fn admit_test_creature(
        &mut self,
        position: MovementLocalPosition,
    ) -> Result<ExactActorRef, CarrierError> {
        self.admit_monster_lab_creature(position, "test:creature", 20)
    }

    /// Test-only parameterized admission for the native monster laboratory.
    #[cfg(test)]
    pub(crate) fn admit_monster_lab_creature(
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

    /// Test only: despawn one actor, so its generation goes stale.
    #[cfg(test)]
    pub(crate) fn remove_test_actor(&mut self, actor: ExactActorRef) -> Result<(), CarrierError> {
        self.carrier.remove(&self.continuity, actor.0).map(|_| ())
    }

    pub(crate) fn reserve_fresh_session(
        &mut self,
        game_session_id: GameSessionId,
    ) -> Result<PlayerActorReservation, CarrierError> {
        self.carrier
            .reserve_player(&self.continuity, game_session_id)
    }

    pub(crate) fn commit_fresh_session(
        &mut self,
        reservation: PlayerActorReservation,
    ) -> Result<ExactActorRef, CarrierError> {
        self.carrier
            .commit_reserved_player(&self.continuity, reservation)
    }

    pub(crate) fn rollback_definitely_uncommitted(
        &mut self,
        reservation: PlayerActorReservation,
    ) -> Result<(), CarrierError> {
        self.carrier
            .rollback_reserved_player(&self.continuity, reservation)
    }

    /// Exact-ref cleanup only. Callers still need an authoritative terminal
    /// GameSession fact; ordinary socket loss is not such a fact.
    pub(crate) fn remove_terminal_session(
        &mut self,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        self.carrier
            .remove_terminal_player(&self.continuity, game_session_id, actor)
    }

    /// A2: bind the admitted session's Character lease to its committed player slot. Binding the
    /// identical lease again is a no-op; a different one conflicts.
    pub(crate) fn bind_attacker_lease(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        lease: CharacterLease,
    ) -> Result<(), CarrierError> {
        self.carrier
            .bind_attacker_lease(&self.continuity, actor.0, game_session_id, lease)
    }

    /// The grace-expiry token of a control-loss epoch.
    pub(crate) const fn grace_expiry_fence(epoch: u64) -> WriteFenceToken {
        WriteFenceToken::ControlLoss(epoch)
    }

    /// The token of a transition id from [`Self::mint_transition_fence`].
    pub(crate) const fn transition_fence(id: u64) -> WriteFenceToken {
        WriteFenceToken::Transition(id)
    }

    /// A fresh transition id for a transition other than grace expiry.
    pub(crate) fn mint_transition_fence(&mut self) -> Result<u64, CarrierError> {
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
    pub(crate) fn fence_player_writes(
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
    pub(crate) fn lift_player_fence(
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
    pub(crate) fn rebind_player_session(
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
    pub(crate) fn bind_continuation(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        reconcile: ContinuationReconcile,
    ) -> Result<(), CarrierError> {
        self.carrier
            .bind_continuation(&self.continuity, actor.0, game_session_id, reconcile)
    }

    /// Test-only census: (committed player actors, pending player reservations).
    #[cfg(test)]
    pub(crate) fn player_slot_counts(&self) -> (usize, usize) {
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
    #[cfg(test)]
    pub(crate) fn entry_start_player_revisions(&self) -> Vec<u64> {
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
    #[cfg(test)]
    pub(crate) fn player_control_loss_epochs(&self) -> Vec<u64> {
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
    #[cfg(test)]
    pub(crate) fn players_positioned_at_entry_start(&self) -> usize {
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

    #[cfg(test)]
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
    fn current_owner_combat_death<'a>(
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
    /// identically to `Slot::Occupied` for position facts, so no further creature-specific branch
    /// is needed for Movement to serve a creature actor generically.
    fn read_movement_position(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.validate_ref(continuity, actor_ref)?;
        self.read_position(continuity, actor_ref)
            .map(MovementPositionSnapshot)
    }

    /// See [`Self::read_movement_position`]: a creature slot commits its cardinal step through
    /// the same compare-commit path a player's does (§4.5).
    fn commit_movement_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        expected: PositionSnapshot,
        next: LocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.validate_ref(continuity, expected.actor_ref)?;
        // An exclusive carrier borrow prevents a slot replacement between this check and
        // the existing private exact-snapshot/revision compare-commit.
        self.compare_commit_position(continuity, expected, expected.version.context, next)
            .map(MovementPositionSnapshot)
    }

    fn current_owner_exact_commit<'a>(
        &'a mut self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerExactActorCommit<'a> {
        CurrentOwnerExactActorCommit {
            carrier: self,
            continuity,
        }
    }

    fn current_owner_exact_lookup<'a>(
        &'a self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerExactActorLookup<'a> {
        CurrentOwnerExactActorLookup {
            carrier: self,
            continuity,
        }
    }

    fn bootstrap_pre_production(
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
            fence_transitions: 0,
            attackers: Vec::new(),
        })
    }

    fn admit(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorState,
    ) -> Result<ActorRef, CarrierError> {
        self.admit_inner(continuity, actor, None, true, None, false)
    }

    /// AI-2 (GAME-AI-01 §4.1: "the carrier's current one-creature limit ... is lifted to this
    /// envelope in AI-2"): more than one creature actor may be admitted per Channel
    /// generation. The general slot-capacity check `admit_inner` already performs is this
    /// method's only capacity gate; population/placement-cell ceilings are a spawn-realization
    /// concern (`realize_spawn`), not this generic per-actor primitive.
    fn admit_creature(
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
        if target_identity.is_empty()
            || target_identity.len() > MAX_OWNER_COMMIT_BINDING_BYTES
            || !target_identity.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'.' | b'_' | b'-' | b'/')
            })
        {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let target_identity = Arc::from(copy_bounded_binding(target_identity.as_bytes())?);
        self.admit_inner(
            continuity,
            actor,
            None,
            true,
            Some((initial_health, target_identity)),
            false,
        )
    }

    fn reserve_player(
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

    fn commit_reserved_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        reservation: PlayerActorReservation,
    ) -> Result<ExactActorRef, CarrierError> {
        let index = self.validate_ref(continuity, reservation.actor_ref)?;
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

    fn remove_terminal_player(
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
            }
        } else {
            Slot::Occupied {
                generation: next_generation,
                actor,
                game_session_id,
                committed,
                position: None,
                lifecycle: Box::default(),
            }
        };
        self.free_head = next_free;
        Ok(actor_ref)
    }

    fn lookup(
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
            Slot::Occupied { .. }
            | Slot::CreatureOccupied { .. }
            | Slot::VacantReusable { .. }
            | Slot::Exhausted { .. } => Err(CarrierError::StaleActorGeneration),
        }
    }

    fn remove(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<ActorState, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        let (generation, actor, removed_creature) = match &self.slots[index] {
            Slot::Occupied {
                generation, actor, ..
            } if *generation == actor_ref.actor_local_generation.0 => (*generation, *actor, false),
            Slot::CreatureOccupied {
                generation, actor, ..
            } if *generation == actor_ref.actor_local_generation.0 => (*generation, *actor, true),
            Slot::Occupied { .. }
            | Slot::CreatureOccupied { .. }
            | Slot::VacantReusable { .. }
            | Slot::Exhausted { .. } => return Err(CarrierError::StaleActorGeneration),
        };
        let free_index =
            u32::try_from(index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        self.slots[index] = Slot::VacantReusable {
            generation,
            next_free: self.free_head,
        };
        self.free_head = Some(free_index);
        // A2: the slot's bound lease and fence go with it.
        self.attackers.retain(|entry| entry.index != index);
        if removed_creature {
            // A retained corpse projection / reward occurrence deliberately outlives this slot
            // (existing Combat D1/D2 behavior: a lost-response retry must still reconcile after
            // administrative removal, `projection_failures_preserve_retry_and_lost_response_idempotency`).
            // `project_committed_lethal_inner`/`reward_occurrence_inner` bound those collections
            // with FIFO eviction instead. Only this actor's *live spawn-cell occupancy* is
            // cleared here, so a respawn's occupancy check does not see a removed actor as still
            // present.
            let removed_actor = ExactActorRef(actor_ref);
            for spawn in &mut self.spawns {
                for cell in &mut spawn.cells {
                    if cell.live == Some(removed_actor) {
                        cell.live = None;
                    }
                }
            }
        }
        Ok(actor)
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
    fn commit_creature_damage_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let OwnerDamageCommand {
            target,
            occurrence,
            binding,
            damage,
        } = command;
        if attacker
            .is_some_and(|attacker| attacker.sub_ordinal >= ABILITY01_EFFECT_PLAN_ENTRIES_MAX)
        {
            return Err(CarrierError::SubOrdinalOutOfRange);
        }
        let index = self.validate_ref(continuity, actor_ref)?;
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
            return Ok(OwnerDamageResult {
                applied: false,
                ..prior.result
            });
        }
        if *health == 0 {
            return Err(CarrierError::CreatureNotActionable);
        }
        if let Some(attacker) = attacker {
            damage_contributors.admission(attacker)?;
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
        let Slot::CreatureOccupied {
            generation,
            target_identity,
            health,
            committed,
            damage_contributors,
            ..
        } = &mut self.slots[index]
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
        *health = next;
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
                Some(attacker.high_water()),
            );
        }
        Ok(result)
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
        if self
            .corpse_projections
            .iter()
            .any(|projection| projection.occurrence.actor == actor)
        {
            return self.existing_corpse_projection(&receipt);
        }
        self.validate_lethal_receipt(continuity, &receipt)?;
        if fail_before_write {
            return Err(CarrierError::InjectedCorpseProjectionFailure);
        }
        let projected = receipt.projection;
        // Bounded FIFO retention (`MAX_RETAINED_CORPSE_PROJECTIONS`): a retained projection
        // deliberately outlives its actor's slot (lost-response replay, above), so this Vec is
        // not otherwise pruned; evict the oldest entry rather than grow without bound. The
        // evicted death's memoized reward occurrence goes with it: a reward occurrence never
        // outlives, and is never evicted independently of, its projection, so a retained
        // projection can never re-mint a second occurrence for the same death.
        self.retain_corpse_projection(projected);
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

    fn retain_corpse_projection(&mut self, projected: RuntimeCorpseProjection) {
        if self.corpse_projections.len() >= MAX_RETAINED_CORPSE_PROJECTIONS {
            let evicted = self.corpse_projections.remove(0).occurrence.actor.0;
            self.death_reward_occurrences
                .retain(|reward| reward.actor != evicted);
        }
        self.corpse_projections.push(projected);
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
    fn initialize_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        context: PreProductionPositionContext,
        position: LocalPosition,
    ) -> Result<PositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
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
    fn read_position(
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
    fn compare_commit_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        expected: PositionSnapshot,
        next_context: PreProductionPositionContext,
        next_position: LocalPosition,
    ) -> Result<PositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, expected.actor_ref)?;
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
        };
        match &mut self.slots[index] {
            Slot::Occupied { position, .. } | Slot::CreatureOccupied { position, .. } => {
                *position = Some(version)
            }
            _ => unreachable!("validated occupied slot"),
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

    fn player_control_loss(
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

    fn record_player_control_loss(
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

    fn restore_player_control(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
        epoch: u64,
    ) -> Result<(), CarrierError> {
        let index = self.player_slot_index(continuity, actor_ref, game_session_id)?;
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
        command: CommandRef,
    ) -> Result<CharacterLease, CarrierError> {
        // The three checks, in order: the slot holds the command's session; it is not fenced;
        // its lease is bound.
        let index =
            self.player_slot_index(continuity, attacker, command.game_session_id())
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

    fn bind_attacker_lease(
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

    fn fence_player_writes(
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

    fn lift_player_fence(
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

    fn rebind_player_session(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        terminal: GameSessionId,
        successor: GameSessionId,
        lease: CharacterLease,
    ) -> Result<(), CarrierError> {
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
        let index = self.player_slot_index(continuity, actor_ref, terminal)?;
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

    fn bind_continuation(
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
        if continuity.world_id != self.world_id
            || continuity.channel_id != self.channel_id
            || continuity.current_generation != self.scope_generation
        {
            return Err(CarrierError::WrongScope);
        }
        Ok(())
    }

    // AI-2 (§4.3, §4.9, D116): spawn realization and respawn.

    /// §4.3: realizes one spawn source at Channel activation: admits one creature per declared
    /// placement cell, up to `definition.population`, each with a fresh actor-local
    /// generation. Every fallible check (duplicate source, the registered
    /// `AI01-SPAWN-SOURCES-PER-SCOPE` ceiling, general slot capacity) precedes any admission
    /// (GAME-AI-01 §6: over budget means zero mutation).
    /// Every fallible check that does not itself mutate `self` (continuity, the position
    /// context, duplicate source, the registered source/population ceilings, general
    /// capacity) precedes any admission. Admission itself is staged: `self.slots`/
    /// `self.free_head` are snapshotted first, and a failure partway through the population
    /// (for example `ActorGenerationExhausted`, the one documented exception that still
    /// mutates a single slot on failure, `admit_inner`) rolls the complete realization back
    /// to that snapshot before returning the error, so a retry never accumulates untracked,
    /// unpositioned creatures (GAME-AI-01 §6: over budget means zero mutation, applied here
    /// to any per-creature failure, not only capacity).
    fn realize_spawn(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        source: SpawnSourceId,
        definition: SpawnDefinition,
        position_context: PreProductionPositionContext,
    ) -> Result<(), CarrierError> {
        self.validate_current_continuity(continuity)?;
        self.validate_position_context(position_context)?;
        if self.spawns.iter().any(|spawn| spawn.source == source) {
            return Err(CarrierError::DuplicateSpawnSource);
        }
        if self.spawns.len() >= AI01_SPAWN_SOURCES_PER_SCOPE_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
        let free_slots = self
            .slots
            .iter()
            .filter(|slot| matches!(slot, Slot::VacantReusable { .. }))
            .count();
        if free_slots < definition.population {
            return Err(CarrierError::CapacityExceeded);
        }
        let target_identity = definition.creature_target_identity.clone();
        let initial_health = definition.creature_initial_health;

        let slots_before = self.slots.clone();
        let free_head_before = self.free_head;
        let mut cells = Vec::with_capacity(definition.placement_cells.len());
        for cell in &definition.placement_cells {
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
                    return Err(error);
                }
            };
            if let Err(error) = self.initialize_position(continuity, actor, position_context, *cell)
            {
                self.slots = slots_before;
                self.free_head = free_head_before;
                return Err(error);
            }
            cells.push(SpawnCellState {
                live: Some(ExactActorRef(actor)),
                attempt: 0,
                successor: 0,
            });
        }
        self.spawns.push(SpawnRealization {
            source,
            definition,
            cells,
        });
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
            _ => false,
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
            .spawns
            .iter()
            .find(|spawn| spawn.source == source)
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
        self.spawns
            .iter()
            .find(|spawn| spawn.source == source)
            .map(|spawn| &spawn.definition)
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
            .spawns
            .iter()
            .position(|spawn| spawn.source == source)
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

        let slots_before = self.slots.clone();
        let free_head_before = self.free_head;
        if let Some(dead_actor) = self.spawns[spawn_index].cells[cell_index].live {
            // Best effort: an already-removed actor (e.g. a repeated call) is not an error here.
            let _ = self.remove(continuity, dead_actor.0);
        }
        let target_identity = self.spawns[spawn_index]
            .definition
            .creature_target_identity
            .clone();
        let initial_health = self.spawns[spawn_index].definition.creature_initial_health;
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
                return Err(error);
            }
        };
        if let Err(error) = self.initialize_position(continuity, actor, position_context, cell) {
            self.slots = slots_before;
            self.free_head = free_head_before;
            return Err(error);
        }
        let state = &mut self.spawns[spawn_index].cells[cell_index];
        state.live = Some(ExactActorRef(actor));
        state.attempt = 0;
        Ok(RespawnResolution::Admitted(ExactActorRef(actor)))
    }
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
    #[cfg(test)]
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

#[cfg(test)]
const TEST_ALLOCATION_FAILURE_CAPACITY: usize = u32::MAX as usize;

/// Foundation-only factory. Path-included Foundation integration crates do not import Content;
/// the Movement library tests supply real Content claims outside this module.
#[cfg(test)]
pub(crate) struct MovementActorFixture {
    owner: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    actor: ExactActorRef,
}

#[cfg(test)]
impl MovementActorFixture {
    pub(crate) fn new(
        position: MovementLocalPosition,
        creature: bool,
    ) -> Result<Self, CarrierError> {
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

    pub(crate) const fn world_id(&self) -> WorldId {
        self.owner.world_id
    }
    pub(crate) const fn actor(&self) -> ExactActorRef {
        self.actor
    }

    pub(crate) fn current_position(&self) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .read_movement_position(&self.owner, self.actor.0)
    }

    pub(crate) fn raw_position(&self) -> Result<(MovementLocalPosition, u64), CarrierError> {
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

    pub(crate) fn borrow_position(&mut self) -> CurrentOwnerMovementPosition<'_> {
        self.carrier.current_owner_movement_position(&self.owner)
    }

    pub(crate) fn wrong_context(&self) -> Result<MovementPositionContext, CarrierError> {
        let mut context = self
            .carrier
            .read_position(&self.owner, self.actor.0)?
            .version
            .context;
        context.coordinate_frame_marker += 1;
        Ok(MovementPositionContext(context))
    }

    pub(crate) fn intervening_write(&mut self) -> Result<(), CarrierError> {
        let expected = self.carrier.read_position(&self.owner, self.actor.0)?;
        self.carrier.compare_commit_position(
            &self.owner,
            expected,
            expected.version.context,
            expected.version.position,
        )?;
        Ok(())
    }

    pub(crate) fn recycle(&mut self) -> Result<(), CarrierError> {
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

    pub(crate) fn become_creature(&mut self) -> Result<(), CarrierError> {
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

    pub(crate) fn advance_owner(&mut self) -> Result<(), CarrierError> {
        self.owner.advance(PreProductionContinuityGrant {
            world_id: self.owner.world_id,
            channel_id: self.owner.channel_id,
            scope_generation: ScopeOwnershipGeneration::new(2)
                .map_err(|_| CarrierError::WrongScope)?,
        })
    }

    pub(crate) fn exhaust_position_revision(&mut self) -> Result<(), CarrierError> {
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
#[cfg(test)]
pub(crate) struct CombatDeathFixture {
    owner: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    actor: ExactActorRef,
    /// D4: `strike_by` derives one fixture command per distinct occurrence text, so the same
    /// text replays as the same `(session, sequence)` command (test-only bookkeeping).
    strike_commands: Vec<(String, u64)>,
}

#[cfg(test)]
impl CombatDeathFixture {
    /// Fixture creature health: test/evidence only, not Reference behavior.
    pub(crate) const HEALTH: i64 = 20;
    const TARGET: &'static str = "fixture:vsl-combat.creature";
    const POSITION: LocalPosition = LocalPosition {
        x: 100,
        y: 100,
        floor: 7,
    };

    pub(crate) fn new(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
    ) -> Result<Self, CarrierError> {
        Self::new_with_health(world_id, channel_id, scope_generation, Self::HEALTH)
    }

    /// Test-only source-health variant; default fixture and all production paths stay intact.
    pub(crate) fn new_with_health(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
        health: i64,
    ) -> Result<Self, CarrierError> {
        let mut owner =
            NamespaceContinuityGuard::from_pre_production_grant(PreProductionContinuityGrant {
                world_id,
                channel_id,
                scope_generation,
            });
        let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1)?;
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
        })
    }

    pub(crate) const fn actor(&self) -> ExactActorRef {
        self.actor
    }

    /// One owner-committed damage occurrence. An identical replay returns the
    /// recorded transition without a second write.
    pub(crate) fn strike(
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

    /// Combat's idempotent projection of the committed lethal occurrence and
    /// the durable death key it names, with the corpse position.
    pub(crate) fn project_death(
        &mut self,
    ) -> Result<(CreatureDeathOccurrenceKey, MovementLocalPosition), CarrierError> {
        let mut combat = self.carrier.current_owner_combat_death(&self.owner);
        let projection = super::exact_actor_test_combat::project_fixed_one_creature_death(
            &mut combat,
            self.actor,
        )?;
        Ok((projection.occurrence().death_key(), projection.position()))
    }

    /// D132/D3-3 + D4: the same owner-committed damage occurrence as [`Self::strike`],
    /// additionally attributed to `attacker`'s running per-creature damage total. The fixture
    /// stands in for the future transport: it maps each distinct `occurrence` text to one
    /// monotonic `CommandId` of one fixed fixture `GameSessionId`, so the same text replays as
    /// the same command and a new text is a new, higher-sequence command.
    pub(crate) fn strike_by(
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
    pub(crate) fn top_damage_character(&mut self) -> Result<Option<CharacterId>, CarrierError> {
        self.carrier
            .current_owner_combat_death(&self.owner)
            .top_damage_character(self.actor)
    }

    /// Administrative despawn: the creature leaves without a semantic death.
    pub(crate) fn despawn(&mut self) -> Result<(), CarrierError> {
        self.carrier.remove(&self.owner, self.actor.0).map(|_| ())
    }

    /// D2b: the same current-owner Combat-death capability
    /// [`ChannelRuntimeV1::borrow_combat_death`] grants, for
    /// `reward_occurrence` tests.
    pub(crate) fn borrow_combat_death(&mut self) -> CurrentOwnerCombatDeath<'_> {
        self.carrier.current_owner_combat_death(&self.owner)
    }

    /// The scope moved: owner continuity advances to `next`, while this
    /// carrier still serves its original generation.
    pub(crate) fn advance_owner(
        &mut self,
        next: ScopeOwnershipGeneration,
    ) -> Result<(), CarrierError> {
        self.owner.advance(PreProductionContinuityGrant {
            world_id: self.owner.world_id,
            channel_id: self.owner.channel_id,
            scope_generation: next,
        })
    }
}

#[cfg(test)]
impl ChannelActorCarrier {
    /// Test only: a committed player slot for `session` with `lease` bound, as fresh admission
    /// leaves it (A2).
    pub(crate) fn admit_bound_test_attacker(
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
    pub(crate) fn test_player_actor(&self, session: GameSessionId) -> Option<ExactActorRef> {
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
#[path = "ability_exact_actor_resolution_tests.rs"]
mod ability_exact_actor_resolution_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "channel_owner_ability_commit_tests.rs"]
mod channel_owner_ability_commit_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "channel_owner_creature_bite_tests.rs"]
mod channel_owner_creature_bite_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "channel_owner_combat_death_tests.rs"]
mod channel_owner_combat_death_tests;

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
#[path = "movement_static_kernel_structural_tests.rs"]
mod movement_static_kernel_structural_tests;

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
            160,
            "session binding must stay inside the already measured fixed-slot footprint"
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
        .expect("D116 definition is within every AI01-SPAWN-* bound")
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

    #[test]
    fn spawn_population_max_accepted_max_plus_one_rejected() {
        assert!(
            SpawnDefinition::new(
                vec![(0, 0, 0), (1, 0, 0), (2, 0, 0), (3, 0, 0)],
                AI01_SPAWN_POPULATION_MAX,
                60_000_000,
                5_000_000,
                "oteryn:creature/rat".to_string(),
                20,
            )
            .is_ok()
        );
        assert_eq!(
            SpawnDefinition::new(
                vec![(0, 0, 0), (1, 0, 0), (2, 0, 0), (3, 0, 0), (4, 0, 0)],
                AI01_SPAWN_POPULATION_MAX + 1,
                60_000_000,
                5_000_000,
                "oteryn:creature/rat".to_string(),
                20,
            ),
            Err(CarrierError::InvalidSpawnDefinition)
        );
    }

    #[test]
    fn spawn_placement_cells_max_accepted_max_plus_one_rejected() {
        let max_cells: Vec<(i32, i32, i16)> = (0..AI01_SPAWN_PLACEMENT_CELLS_MAX as i32)
            .map(|x| (x, 0, 0))
            .collect();
        assert!(
            SpawnDefinition::new(
                max_cells,
                AI01_SPAWN_PLACEMENT_CELLS_MAX,
                60_000_000,
                5_000_000,
                "oteryn:creature/rat".to_string(),
                20,
            )
            .is_ok()
        );
        let over_cells: Vec<(i32, i32, i16)> = (0..=AI01_SPAWN_PLACEMENT_CELLS_MAX as i32)
            .map(|x| (x, 0, 0))
            .collect();
        assert_eq!(
            SpawnDefinition::new(
                over_cells,
                AI01_SPAWN_PLACEMENT_CELLS_MAX + 1,
                60_000_000,
                5_000_000,
                "oteryn:creature/rat".to_string(),
                20,
            ),
            Err(CarrierError::InvalidSpawnDefinition)
        );
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

    #[test]
    fn spawn_sources_per_scope_max_accepted_max_plus_one_rejected() {
        let (continuity, mut carrier) = carrier(AI01_SPAWN_SOURCES_PER_SCOPE_MAX + 1);
        let context = spawn_position_context(&continuity);
        let one_cell = |seed: i32| {
            SpawnDefinition::new(
                vec![(seed, 0, 0)],
                1,
                60_000_000,
                5_000_000,
                "oteryn:creature/rat".to_string(),
                20,
            )
            .expect("valid one-cell definition")
        };
        for index in 0..AI01_SPAWN_SOURCES_PER_SCOPE_MAX {
            carrier
                .realize_spawn(
                    &continuity,
                    SpawnSourceId(index as u16),
                    one_cell(index as i32),
                    context,
                )
                .expect("within the registered ceiling");
        }
        assert_eq!(
            carrier.realize_spawn(
                &continuity,
                SpawnSourceId(AI01_SPAWN_SOURCES_PER_SCOPE_MAX as u16),
                one_cell(AI01_SPAWN_SOURCES_PER_SCOPE_MAX as i32),
                context,
            ),
            Err(CarrierError::CapacityExceeded)
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
}

/// A2 (#1635): the bound attacker slot and its write fence (CHARM-DESC-FENCE-LEASE §6 item 3).
#[cfg(test)]
#[allow(clippy::expect_used)]
mod attacker_fence_tests {
    use super::super::exact_actor_test_ability::commit::{
        OwnerCharmDamagePlan, OwnerCommitError, commit_exact_owner_charm_damage,
        commit_exact_owner_damage, commit_exact_owner_primary_damage,
    };
    use super::super::exact_actor_test_ability::exact_actor_resolution::{
        ExactActorProposal, ResolvedExactActor, resolve_exact_actor,
    };
    use super::super::exact_actor_test_ability::{
        AbilityIntent, AbilityOccurrence, CalculationStage, CommitGroup, Effect, EffectPlan,
        ProposalSource, RevisionSet,
    };
    use super::*;

    const SUPERSEDED: Result<OwnerDamageResult, CarrierError> =
        Err(CarrierError::SupersededAttackerSession);

    fn id(seed: u8) -> [u8; 16] {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        bytes
    }

    fn session(seed: u8) -> GameSessionId {
        GameSessionId::decode(&id(seed)).expect("session")
    }

    fn lease(character: u8, generation: u64) -> CharacterLease {
        CharacterLease::new(
            CharacterId::decode(&id(character)).expect("character"),
            generation,
        )
        .expect("lease")
    }

    fn command(session_seed: u8, sequence: u64) -> CommandRef {
        CommandRef::new(
            session(session_seed),
            super::super::CommandId::new(sequence).expect("command"),
        )
    }

    struct Fixture {
        continuity: NamespaceContinuityGuard,
        carrier: ChannelActorCarrier,
        creature: ExactActorRef,
        /// Session 1 of Character 1, bound at lease generation 1.
        attacker: ExactActorRef,
    }

    fn fixture(seed: u64) -> Fixture {
        let mut continuity =
            NamespaceContinuityGuard::from_pre_production_grant(PreProductionContinuityGrant {
                world_id: WorldId::decode(&id(seed as u8)).expect("world"),
                channel_id: ChannelId::decode(&id(seed as u8 + 1)).expect("channel"),
                scope_generation: ScopeOwnershipGeneration::new(1).expect("scope"),
            });
        let mut carrier =
            ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 3).expect("carrier");
        let creature = ExactActorRef(
            carrier
                .admit_creature(&continuity, ActorState(1), "target:one", 100)
                .expect("creature"),
        );
        let attacker = carrier
            .admit_bound_test_attacker(&continuity, session(1), lease(1, 1))
            .expect("bound attacker");
        Fixture {
            continuity,
            carrier,
            creature,
            attacker,
        }
    }

    impl Fixture {
        fn hit(
            &mut self,
            attacker: ExactActorRef,
            command: CommandRef,
        ) -> Result<OwnerDamageResult, CarrierError> {
            let binding = format!("hit:{}", command.command_id().get());
            self.carrier
                .current_owner_exact_commit(&self.continuity)
                .commit_damage_for_bound_attacker(
                    self.creature,
                    attacker,
                    command,
                    0,
                    OwnerDamageCommand {
                        target: b"target:one",
                        occurrence: &[],
                        binding: binding.as_bytes(),
                        damage: 1,
                    },
                )
        }

        fn health(&self) -> i64 {
            match &self.carrier.slots[0] {
                Slot::CreatureOccupied { health, .. } => Some(*health),
                _ => None,
            }
            .expect("the creature is slot 0")
        }

        fn fence(&mut self, token: WriteFenceToken) -> Result<WriteFenceSet, CarrierError> {
            self.carrier
                .fence_player_writes(&self.continuity, self.attacker.0, session(1), token)
        }

        fn lift(&mut self, token: WriteFenceToken) -> Result<bool, CarrierError> {
            self.carrier
                .lift_player_fence(&self.continuity, self.attacker.0, session(1), token)
        }

        fn lose_control(&mut self, epoch: u64) {
            self.carrier
                .record_player_control_loss(
                    &self.continuity,
                    self.attacker.0,
                    session(1),
                    ControlLossMark {
                        epoch,
                        grace_deadline: 100,
                    },
                )
                .expect("control loss");
        }
    }

    #[test]
    fn a_write_between_fence_and_commit_is_refused_and_only_the_exact_token_lifts() {
        let mut f = fixture(10);
        assert!(f.hit(f.attacker, command(1, 1)).is_ok());
        let token = WriteFenceToken::Transition(7);
        assert_eq!(f.fence(token), Ok(WriteFenceSet::Fenced));
        let before = f.carrier.slots.clone();
        assert_eq!(f.hit(f.attacker, command(1, 2)), SUPERSEDED);
        assert_eq!(f.carrier.slots, before);
        // A lift with any other token is a no-op.
        assert_eq!(f.lift(WriteFenceToken::Transition(8)), Ok(false));
        assert_eq!(f.lift(WriteFenceToken::ControlLoss(7)), Ok(false));
        assert_eq!(f.hit(f.attacker, command(1, 2)), SUPERSEDED);
        // NotApplicable with the session still holding the lease: the exact token lifts.
        assert_eq!(f.lift(token), Ok(true));
        assert_eq!(f.lift(token), Ok(false));
        assert!(f.hit(f.attacker, command(1, 2)).is_ok());
        assert_eq!(f.health(), 98);
    }

    #[test]
    fn one_transition_per_session_joins_its_retry_and_never_replaces_a_fence() {
        let mut f = fixture(20);
        f.lose_control(3);
        let grace = WriteFenceToken::ControlLoss(3);
        assert_eq!(f.fence(grace), Ok(WriteFenceSet::Fenced));
        // An unknown durable outcome keeps the fence; the retry joins it.
        assert_eq!(f.fence(grace), Ok(WriteFenceSet::Joined));
        // An overlapping logout or revocation waits for the first transition's step (c).
        assert_eq!(
            f.fence(WriteFenceToken::Transition(1)),
            Err(CarrierError::WriteFenceBusy)
        );
        assert_eq!(f.hit(f.attacker, command(1, 1)), SUPERSEDED);
        // After a lift, the second transition starts again at step (a) with its own token.
        assert_eq!(f.lift(grace), Ok(true));
        assert_eq!(
            f.fence(WriteFenceToken::Transition(1)),
            Ok(WriteFenceSet::Fenced)
        );
        assert_eq!(f.fence(grace), Err(CarrierError::WriteFenceBusy));
        // After a terminal settle, the second transition finds nothing to do.
        f.carrier
            .remove_terminal_player(&f.continuity, session(1), f.attacker)
            .expect("terminal settle");
        assert!(f.fence(WriteFenceToken::Transition(2)).is_err());
        assert!(f.lift(WriteFenceToken::Transition(1)).is_err());
        assert_eq!(f.hit(f.attacker, command(1, 1)), SUPERSEDED);
    }

    #[test]
    fn grace_expiry_fences_only_its_marked_epoch_and_a_resume_lifts_it() {
        let mut f = fixture(30);
        // No mark, or another epoch: fencing refuses and the transition does not start.
        assert_eq!(
            f.fence(WriteFenceToken::ControlLoss(1)),
            Err(CarrierError::ControlLossConflict)
        );
        f.lose_control(1);
        assert_eq!(
            f.fence(WriteFenceToken::ControlLoss(2)),
            Err(CarrierError::ControlLossConflict)
        );
        assert_eq!(
            f.fence(WriteFenceToken::ControlLoss(1)),
            Ok(WriteFenceSet::Fenced)
        );
        assert_eq!(f.hit(f.attacker, command(1, 1)), SUPERSEDED);
        // The same-session resume lifts the fence together with the mark of its epoch.
        f.carrier
            .restore_player_control(&f.continuity, f.attacker.0, session(1), 1)
            .expect("resume");
        assert!(f.hit(f.attacker, command(1, 1)).is_ok());
        // A resume never lifts another transition's fence.
        f.lose_control(2);
        assert_eq!(
            f.fence(WriteFenceToken::Transition(5)),
            Ok(WriteFenceSet::Fenced)
        );
        f.carrier
            .restore_player_control(&f.continuity, f.attacker.0, session(1), 2)
            .expect("resume");
        assert_eq!(f.hit(f.attacker, command(1, 2)), SUPERSEDED);
    }

    #[test]
    fn a_terminal_session_is_never_unfenced_and_its_slot_binding_goes_with_it() {
        let mut f = fixture(40);
        let token = WriteFenceToken::Transition(1);
        assert_eq!(f.fence(token), Ok(WriteFenceSet::Fenced));
        f.carrier
            .remove_terminal_player(&f.continuity, session(1), f.attacker)
            .expect("terminal settle");
        assert!(f.lift(token).is_err());
        assert!(
            f.carrier
                .bind_attacker_lease(&f.continuity, f.attacker.0, session(1), lease(1, 1))
                .is_err()
        );
        assert_eq!(f.hit(f.attacker, command(1, 1)), SUPERSEDED);
        assert!(f.carrier.attackers.is_empty());
        // A new session in the recycled slot inherits neither the fence nor the binding.
        let reservation = f
            .carrier
            .reserve_player(&f.continuity, session(2))
            .expect("reserve");
        let next = f
            .carrier
            .commit_reserved_player(&f.continuity, reservation)
            .expect("commit");
        assert_eq!(f.hit(next, command(2, 1)), SUPERSEDED);
        f.carrier
            .bind_attacker_lease(&f.continuity, next.0, session(2), lease(1, 2))
            .expect("bind");
        assert_eq!(
            f.carrier
                .bind_attacker_lease(&f.continuity, next.0, session(2), lease(1, 3)),
            Err(CarrierError::AttackerBindingConflict)
        );
        assert!(f.hit(next, command(2, 1)).is_ok());
    }

    #[test]
    fn a_rebind_drops_the_old_fence_and_leaves_the_rebound_slot_unfenced() {
        let mut f = fixture(50);
        f.lose_control(4);
        assert_eq!(
            f.fence(WriteFenceToken::ControlLoss(4)),
            Ok(WriteFenceSet::Fenced)
        );
        for (successor, refused) in [
            (lease(1, 1), "an equal lease generation"),
            (lease(2, 2), "another Character"),
        ] {
            assert_eq!(
                f.carrier.rebind_player_session(
                    &f.continuity,
                    f.attacker.0,
                    session(1),
                    session(2),
                    successor
                ),
                Err(CarrierError::AttackerBindingConflict),
                "{refused}"
            );
        }
        f.carrier
            .rebind_player_session(
                &f.continuity,
                f.attacker.0,
                session(1),
                session(2),
                lease(1, 2),
            )
            .expect("post-grace rebind");
        assert_eq!(f.hit(f.attacker, command(1, 1)), SUPERSEDED);
        assert!(f.hit(f.attacker, command(2, 1)).is_ok());
        assert!(f.fence(WriteFenceToken::ControlLoss(4)).is_err());
        assert_eq!(
            f.carrier
                .player_control_loss(&f.continuity, f.attacker.0, session(2)),
            Ok(None)
        );
    }

    #[test]
    fn a_same_session_continuation_reconciles_before_it_binds() {
        let mut f = fixture(60);
        let reservation = f
            .carrier
            .reserve_player(&f.continuity, session(3))
            .expect("reserve");
        let slot = f
            .carrier
            .commit_reserved_player(&f.continuity, reservation)
            .expect("reconstructed slot");
        // No write is admitted before the binding exists.
        assert_eq!(f.hit(slot, command(3, 1)), SUPERSEDED);
        for refused in [
            ContinuationReconcile::Terminal,
            ContinuationReconcile::Unprovable,
        ] {
            assert_eq!(
                f.carrier
                    .bind_continuation(&f.continuity, slot.0, session(3), refused),
                Err(CarrierError::ContinuationRefused)
            );
            assert_eq!(f.hit(slot, command(3, 1)), SUPERSEDED);
        }
        f.carrier
            .bind_continuation(
                &f.continuity,
                slot.0,
                session(3),
                ContinuationReconcile::Proven(lease(3, 1)),
            )
            .expect("proven continuation binds unfenced");
        assert!(f.hit(slot, command(3, 1)).is_ok());
        assert_eq!(
            f.carrier.bind_continuation(
                &f.continuity,
                slot.0,
                session(3),
                ContinuationReconcile::Proven(lease(3, 2)),
            ),
            Err(CarrierError::ContinuationRefused)
        );
    }

    fn primary_plan(f: &Fixture) -> (ResolvedExactActor, EffectPlan) {
        let occurrence = AbilityOccurrence::new(
            "attack:rebind",
            RevisionSet::new(
                "rules:1",
                "content:1",
                "world:1",
                "formula:1",
                "simulation:1",
            )
            .expect("revisions"),
        )
        .expect("occurrence");
        let resolved = resolve_exact_actor(
            &f.carrier.current_owner_exact_lookup(&f.continuity),
            &occurrence,
            ExactActorProposal::client(f.creature),
        )
        .expect("current creature");
        let plan = EffectPlan::immediate(
            occurrence,
            AbilityIntent::normalize(ProposalSource::Client, "actor:fixture", &["target:one"])
                .expect("intent"),
            vec![Effect::damage("target:one", 3).expect("damage")],
            vec![],
            CommitGroup::atomic("scope:fixture", "group:one").expect("group"),
        )
        .expect("plan");
        (resolved, plan)
    }

    fn charm_plan(parent: &EffectPlan) -> EffectPlan {
        EffectPlan::ordered_sequential(
            parent.occurrence().clone(),
            parent.intent().clone(),
            vec![
                parent.effects()[0].clone(),
                Effect::damage("target:one", 9).expect("generated"),
            ],
            vec![CalculationStage::new("charm:generated").expect("stage")],
            parent.commit_group().owner_scope(),
            parent.commit_group().group_id(),
        )
        .expect("charm plan")
    }

    #[test]
    fn a_rebind_during_an_in_flight_charm_or_descriptor_commit_settles_exactly_once() {
        for charm_before_rebind in [false, true] {
            let mut f = fixture(70);
            let (resolved, plan) = primary_plan(&f);
            let parent = commit_exact_owner_primary_damage(
                &mut f.carrier.current_owner_exact_commit(&f.continuity),
                &resolved,
                &plan,
                f.attacker,
                command(1, 1),
            )
            .expect("primary under the bound lease");
            let frozen = OwnerCharmDamagePlan::prepare(&parent, charm_plan(&plan))
                .expect("prepare")
                .expect("live parent");
            let charm = |f: &mut Fixture| {
                commit_exact_owner_charm_damage(
                    &mut f.carrier.current_owner_exact_commit(&f.continuity),
                    &frozen,
                    f.attacker,
                    command(1, 1),
                )
                .map(|charm| charm.result)
            };
            if charm_before_rebind {
                assert!(charm(&mut f).expect("descendant").applied);
            }
            let settled = f.health();
            assert_eq!(settled, if charm_before_rebind { 88 } else { 97 });
            f.carrier
                .rebind_player_session(
                    &f.continuity,
                    f.attacker.0,
                    session(1),
                    session(2),
                    lease(1, 2),
                )
                .expect("rebind");
            // Neither the descendant nor a replay of the primary applies again under the old
            // session: each occurrence settled exactly once, before the rebind or never.
            assert_eq!(
                charm(&mut f),
                Err(OwnerCommitError::Owner(
                    CarrierError::SupersededAttackerSession
                ))
            );
            assert_eq!(
                commit_exact_owner_damage(
                    &mut f.carrier.current_owner_exact_commit(&f.continuity),
                    &resolved,
                    &plan,
                    f.attacker,
                    command(1, 1),
                ),
                Err(OwnerCommitError::Owner(
                    CarrierError::SupersededAttackerSession
                ))
            );
            assert_eq!(f.health(), settled);
        }
    }

    #[test]
    fn a_fenced_slot_refuses_every_bridge_before_the_owner_write() {
        let mut f = fixture(80);
        let (resolved, plan) = primary_plan(&f);
        f.lose_control(1);
        assert_eq!(
            f.fence(WriteFenceToken::ControlLoss(1)),
            Ok(WriteFenceSet::Fenced)
        );
        let before = f.carrier.slots.clone();
        assert_eq!(
            commit_exact_owner_primary_damage(
                &mut f.carrier.current_owner_exact_commit(&f.continuity),
                &resolved,
                &plan,
                f.attacker,
                command(1, 1),
            )
            .map(|_| ()),
            Err(OwnerCommitError::Owner(
                CarrierError::SupersededAttackerSession
            ))
        );
        // A command of a session the slot does not hold is refused the same way.
        assert_eq!(
            commit_exact_owner_damage(
                &mut f.carrier.current_owner_exact_commit(&f.continuity),
                &resolved,
                &plan,
                f.attacker,
                command(9, 1),
            ),
            Err(OwnerCommitError::Owner(
                CarrierError::SupersededAttackerSession
            ))
        );
        assert_eq!(f.carrier.slots, before);
    }

    #[test]
    fn fence_tokens_keep_their_kind() {
        assert_eq!(
            ChannelRuntimeV1::transition_fence(2),
            WriteFenceToken::Transition(2)
        );
        assert_eq!(
            ChannelRuntimeV1::grace_expiry_fence(2),
            WriteFenceToken::ControlLoss(2)
        );
        assert_ne!(
            ChannelRuntimeV1::transition_fence(2),
            ChannelRuntimeV1::grace_expiry_fence(2)
        );
    }
}
