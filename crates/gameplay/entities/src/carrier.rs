//! Fixed-bound, pre-production-only Channel actor storage.
//!
//! New mechanic code belongs in its own `carrier/<topic>.rs` module, not here.
//!
//! The module stays crate-private. Its only continuity grant is derived by
//! `ChannelRuntimeV1::from_committed_assignment` from the committed Channel
//! assignment that `serve` consumes before readiness; one runtime per
//! ownership generation is enforced by that composition, not by this module.

use oteryn_gameplay_foundation::foundation::{
    ChannelId, CharacterId, CharacterLease, CommandRef, GameSessionId, NodeId,
    ScopeOwnershipGeneration, WorldId,
};
pub use {
    oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    oteryn_gameplay_domain::domain::carrier_values::{CarrierError, LocalPosition},
    oteryn_gameplay_foundation::foundation::{
        ActorLocalGeneration, ActorLocalId, ActorRef, ExactActorRef,
    },
};
/// Carrier entry points the crate's runtime uses and that the lifted qualification tests
/// drive directly: `pub` only in test builds, crate-private everywhere else, so no other
/// crate can admit actors, read owner commits or advance continuity in production.
macro_rules! test_support_pub {
    ($(#[$meta:meta])* fn $($rest:tt)*) => {
        #[cfg(any(test, feature = "test-support"))]
        $(#[$meta])* pub fn $($rest)*
        #[cfg(not(any(test, feature = "test-support")))]
        $(#[$meta])* pub(crate) fn $($rest)*
    };
}

mod actor_memos;

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[path = "carrier/area_heal_batch_native_tests.rs"]
mod area_heal_batch_native_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "carrier/channel_actor_position_tests.rs"]
mod channel_actor_position_tests;

mod continuity;

mod corpse_views;

mod creature_heal;

mod creature_relocation;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "carrier/damage_contributors_tests.rs"]
mod damage_contributors_tests;

mod damage_ledger;

#[path = "carrier/encounter_access.rs"]
pub mod encounter_access;

#[cfg(any(test, feature = "test-support"))]
mod fixtures;

#[path = "carrier/monster_summon_admission.rs"]
mod monster_summon_admission;

mod native_encounter;

mod native_summon;

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[path = "carrier/native_summon_reconciliation_tests.rs"]
mod native_summon_reconciliation_tests;

#[path = "carrier/npc_actor.rs"]
pub mod npc_actor;

mod owner_views;

#[cfg(test)]
#[path = "carrier/party_reward_occurrence_tests.rs"]
mod party_reward_occurrence_tests;

mod player_control;

mod positions;

#[path = "carrier/runtime_actor_companion.rs"]
pub mod runtime_actor_companion;

#[path = "carrier/runtime_actor_conditions.rs"]
mod runtime_actor_conditions;

#[path = "carrier/runtime_actor_damage_batch.rs"]
mod runtime_actor_damage_batch;

#[path = "carrier/runtime_actor_death.rs"]
mod runtime_actor_death;

#[path = "carrier/runtime_actor_familiar_defense.rs"]
pub mod runtime_actor_familiar_defense;

#[path = "carrier/runtime_actor_party.rs"]
pub mod runtime_actor_party;

#[path = "carrier/runtime_actor_periodic.rs"]
pub mod runtime_actor_periodic;

#[path = "carrier/runtime_actor_reentry.rs"]
pub mod runtime_actor_reentry;

#[path = "carrier/runtime_actor_source_reservation.rs"]
pub mod runtime_actor_source_reservation;

#[path = "carrier/runtime_actor_source_step.rs"]
pub mod runtime_actor_source_step;

#[path = "carrier/runtime_actor_spell.rs"]
pub mod runtime_actor_spell;

#[path = "carrier/runtime_actor_spell_types.rs"]
pub mod runtime_actor_spell_types;

// SPAWN-1a: the pure spawn schedule (std only) compiles with the carrier so every
// path-included Foundation crate has it without the `ai` tree.
#[path = "carrier/spawn.rs"]
mod spawn;

mod spawn_realization;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "carrier/tests.rs"]
mod tests;

use std::collections::BTreeSet;
use std::mem::size_of;
use std::sync::Arc;

#[allow(unused_imports)] // The Foundation facade is composed by the owning consumer child.
pub use runtime_actor_conditions::{
    ActorConditionPlan, ActorConditionTickPlan, ActorConditionTransition, ApplicationFacts,
    AttributeModifier, AttributeModifiers, CombatSkill, ConditionDefinition, ConditionOwnerError,
    ConditionRefusal, ConditionSource, ConditionSourceKind, ConditionStore, ConditionTick,
    ConditionType, ConditionValues, DamageSchedule, DamageSegment, DotElement, ExactSpeedRatio,
    RationalSpeedRange, SpeedRange, StatusKind, TickFacts, TickKind,
};
use runtime_actor_conditions::{CreatureCommitState, PlayerRuntimeState};

pub use actor_memos::ActorMemoTable;
use corpse_views::CorpseItemBinding;
pub use corpse_views::RuntimeCorpseItem;
pub use corpse_views::VisibleRuntimeCorpse;
pub use creature_heal::CreatureSelfHealLedger;
pub use creature_heal::CreatureSelfHealRegistration;
pub use damage_ledger::ABILITY01_EFFECT_PLAN_ENTRIES_MAX;
pub use damage_ledger::COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX;
pub use damage_ledger::COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX;
pub use damage_ledger::DamageContributor;
pub use damage_ledger::DamageContributors;
pub use damage_ledger::DamageOrigin;
pub use damage_ledger::DamageReceipts;
use damage_ledger::HighWater;
pub use damage_ledger::RewardDamageShare;
pub use damage_ledger::project_fixed_one_creature_death;
#[cfg(any(test, feature = "test-support"))]
pub use fixtures::CombatDeathFixture;
#[cfg(any(test, feature = "test-support"))]
pub use fixtures::MovementActorFixture;
#[cfg(any(test, feature = "test-support"))]
use fixtures::TEST_ALLOCATION_FAILURE_CAPACITY;
pub use native_encounter::EncounterSpawnError;
pub use native_encounter::NativeEncounterSpawnSpec;
pub use native_summon::NativeSummonAdmissionSpec;
pub use native_summon::NativeSummonDeath;
use native_summon::NativeSummonLink;
use native_summon::NativeSummonLinks;
pub use native_summon::NativeSummonMovementOutcome;
pub use native_summon::NativeSummonOrigin;
use native_summon::copy_native_summon_key;
pub use spawn_realization::AI01_PENDING_RESPAWN_TIMERS_PER_DEAD_ACTOR;
pub use spawn_realization::ActivationSpawnFacts;
pub use spawn_realization::ActivationSpawnPointFacts;
pub use spawn_realization::ActivationSpawnSourceFacts;
pub use spawn_realization::RespawnFamily;
pub use spawn_realization::RespawnOccurrence;
pub use spawn_realization::RespawnResolution;
use spawn_realization::SpawnActivation;
pub use spawn_realization::SpawnActivationProgress;
pub use spawn_realization::SpawnDefinition;
pub use spawn_realization::SpawnDriveReport;
use spawn_realization::SpawnDriveWindow;
use spawn_realization::SpawnRealization;
pub use spawn_realization::SpawnSourceId;

/// Authority supplied by a future, independently accepted assignment consumer.
///
/// There is deliberately no production constructor, issuer, reissuer, `Clone`, or `Copy`
/// implementation here. Consuming this value makes namespace bootstrap one-shot. Its fields,
/// the continuity guard's scope fields and the commit borrow's carrier stay sealed behind the
/// [`ChannelRuntimeV1`] facade, so no other crate can mint continuity for a retired runtime:
///
/// ```
/// use oteryn_gameplay_entities::ChannelRuntimeV1;
/// use oteryn_gameplay_entities::carrier::{
///     CurrentOwnerExactActorCommit, NamespaceContinuityGuard, PreProductionContinuityGrant,
/// };
///
/// fn facade(runtime: &mut ChannelRuntimeV1) -> CurrentOwnerExactActorCommit<'_> {
///     let _ = runtime.binding().world_id();
///     runtime.borrow_exact_actor_commit()
/// }
/// fn named(_: &PreProductionContinuityGrant, _: &NamespaceContinuityGuard) {}
/// ```
///
/// ```compile_fail
/// use oteryn_gameplay_entities::ChannelRuntimeV1;
/// use oteryn_gameplay_entities::carrier::PreProductionContinuityGrant;
///
/// fn forge(runtime: &ChannelRuntimeV1) -> PreProductionContinuityGrant {
///     let binding = runtime.binding();
///     PreProductionContinuityGrant {
///         world_id: binding.world_id(),
///         channel_id: binding.channel_id(),
///         scope_generation: binding.scope_generation(),
///     }
/// }
/// ```
///
/// ```compile_fail
/// use oteryn_gameplay_foundation::foundation::ScopeOwnershipGeneration;
/// use oteryn_gameplay_entities::carrier::NamespaceContinuityGuard;
///
/// fn rewind(guard: &mut NamespaceContinuityGuard, generation: ScopeOwnershipGeneration) {
///     guard.current_generation = generation;
/// }
/// ```
///
/// ```compile_fail
/// use oteryn_gameplay_entities::ChannelRuntimeV1;
///
/// fn escape(runtime: &mut ChannelRuntimeV1) {
///     let commit = runtime.borrow_exact_actor_commit();
///     let _carrier = commit.carrier;
/// }
/// ```
#[derive(Debug)]
pub struct PreProductionContinuityGrant {
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
pub struct NamespaceContinuityGuard {
    world_id: WorldId,
    channel_id: ChannelId,
    current_generation: ScopeOwnershipGeneration,
    current_generation_claimed: bool,
    active: bool,
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
    carrier: &'a mut ChannelActorCarrier,
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

pub use oteryn_gameplay_domain::domain::charm_data::OwnerDamageResult;

/// Immutable, borrowed Ability proposal. Authority is checked by the owner,
/// independently of these bytes, before the single slot replacement.
#[derive(Clone, Copy)]
pub struct OwnerDamageCommand<'a> {
    pub target: &'a [u8],
    pub occurrence: &'a [u8],
    pub binding: &'a [u8],
    pub damage: i64,
}

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
    pub(crate) binding: Box<[u8]>,
    damage: i64,
    pub(crate) result: OwnerDamageResult,
    /// D142: this creature generation's owner damage-application ordinal of this receipt.
    pub(crate) ordinal: u64,
    /// D141: `Some` for an attributed commit (its replay identity), `None` for an unsequenced
    /// one, which is never evictable.
    pub(crate) origin: Option<DamageOrigin>,
}

#[cfg(any(test, feature = "test-support"))]
impl OwnerCommitRecord {
    /// Test builds only: the complete commit binding of this receipt.
    pub fn binding(&self) -> &[u8] {
        &self.binding
    }

    /// Test builds only: the owner damage result this receipt retained.
    pub const fn result(&self) -> &OwnerDamageResult {
        &self.result
    }

    /// Test builds only: this receipt's owner damage-application ordinal.
    pub const fn ordinal(&self) -> u64 {
        self.ordinal
    }

    /// Test builds only: this receipt's replay identity, if attributed.
    pub const fn origin(&self) -> Option<DamageOrigin> {
        self.origin
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
        ExactActorRef(self.0).scope_generation()
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
    pub(crate) projection: RuntimeCorpseProjection,
}

/// 4096 is the registered Ability plan bound; the owner still independently
/// checks the actual complete encoding before allocating its one receipt.
pub const MAX_OWNER_COMMIT_BINDING_BYTES: usize = 4_096;

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
    pub(crate) scope_generation: ScopeOwnershipGeneration,
    pub coordinate_frame_marker: u64,
    pub map_revision_marker: u64,
    pub content_generation_marker: u64,
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
    /// once per (dead actor, Character), up to 50 principals per death (D109).
    /// Entries share the bound and pruning of `corpse_projections`. A
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
    encounter_hooks: Option<Box<dyn encounter_access::EncounterHooks>>,
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
/// the source encounter ledger, which no production owner constructs yet.
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

/// Outcome of a first-entry position initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstEntryPosition {
    /// This call wrote the start position.
    Initialized(MovementPositionSnapshot),
    /// The same initialization had already completed; nothing was written.
    Reconciled(MovementPositionSnapshot),
}

/// The owner cycle an independently committed assignment installs. Only
/// [`ChannelRuntimeV1::from_committed_assignment`] builds one, once per runtime.
struct CommittedAssignmentOwnerCycle {
    world_id: WorldId,
    channel_id: ChannelId,
    scope_generation: ScopeOwnershipGeneration,
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
    owner_cycle: oteryn_gameplay_foundation::foundation::ScopeRuntimeFence,
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
impl ChannelRuntimeV1 {
    /// Issue from the one non-clonable owner cycle installed by the independently committed
    /// assignment. A retired namespace can neither schedule nor commit further actor work.
    pub fn issue_owner_work(
        &mut self,
    ) -> Result<oteryn_gameplay_foundation::foundation::RuntimeWorkStamp, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        let ordinal = self
            .owner_cycle
            .accept_input(self.binding.scope_generation)
            .map_err(|_| CarrierError::WrongScope)?;
        Ok(self.owner_cycle.stamp(ordinal))
    }

    pub fn owner_fence(
        &self,
    ) -> Result<&oteryn_gameplay_foundation::foundation::ScopeRuntimeFence, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        if !self.owner_cycle.is_current_for_scope(
            oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
                self.binding.world_id,
                self.binding.channel_id,
            ),
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
        if content.world_id() != world_id {
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
            owner_cycle:
                oteryn_gameplay_foundation::foundation::ScopeRuntimeFence::from_owner_cycle_grant(
                    CommittedAssignmentOwnerCycle {
                        world_id,
                        channel_id,
                        scope_generation,
                    },
                ),
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
    ///
    /// SUMMON-OWNER-END-1 (CREATURE-AI-0 §8.4): the owner's summons, familiars included, leave
    /// with it, as a removal with no death; no familiar durable state is written here.
    pub fn remove_terminal_session(
        &mut self,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        if self
            .carrier
            .contains_committed_player(&self.continuity, game_session_id, actor)
        {
            self.remove_owned_companions(actor, game_session_id)?;
        }
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
            encounter_hooks: None,
            fence_transitions: 0,
            attackers: Vec::new(),
            occupied,
            corpse_item_bindings: Vec::new(),
            corpse_projection_pins: Vec::new(),
        })
    }

    test_support_pub! {
        fn admit(
            &mut self,
            continuity: &NamespaceContinuityGuard,
            actor: ActorState,
        ) -> Result<ActorRef, CarrierError> {
            self.admit_inner(continuity, actor, None, true, None, false)
        }
    }

    test_support_pub! {
        /// AI-2 (GAME-AI-01 §4.1: "the carrier's current one-creature limit ... is lifted to this
        /// envelope in AI-2"): more than one creature actor may be admitted per Channel
        /// generation. Its capacity gates are `CREATUREAI0-RL-04`
        /// ([`CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX`]) and the general slot-capacity check
        /// `admit_inner` performs; population/placement-cell ceilings are a spawn-realization
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
        let actor_ref = ActorRef::from_owner_parts(
            self.world_id,
            self.channel_id,
            self.scope_generation,
            actor_local_id,
            ActorLocalGeneration(next_generation),
        );

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

    fn validate_ref(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<usize, CarrierError> {
        self.validate_current_continuity(continuity)?;
        if actor_ref.world_id != continuity.world_id
            || actor_ref.channel_id != continuity.channel_id
            || ExactActorRef(actor_ref).scope_generation() != continuity.current_generation
            || actor_ref.world_id != self.world_id
            || actor_ref.channel_id != self.channel_id
            || ExactActorRef(actor_ref).scope_generation() != self.scope_generation
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
