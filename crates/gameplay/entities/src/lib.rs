//! L2 kernel: actor carrier and component registry with deterministic ordering.
//!
//! The physical Channel actor carrier composes the Foundation kernel and qualified Content.

#[allow(unused_imports)]
use oteryn_gameplay_domain::domain;
#[allow(unused_imports)]
pub use oteryn_gameplay_foundation::foundation::*;

pub mod content_adapters;

#[allow(dead_code)]
pub mod carrier;
pub use carrier as runtime_actor_carrier;
pub use runtime_actor_carrier::npc_actor::{
    NpcPlacement, NpcPlacementError, NpcPlacementTable, NpcPlacements,
};
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_companion::{
    CompanionMaster, CompanionSnapshot, CompanionState, FamiliarDefenseClock,
    PreparedCompanionAssignment, PreparedCompanionSpawn, PreparedCreatureProfileTransformation,
};
pub use runtime_actor_carrier::runtime_actor_source_step::{
    SourceStepCommitProof, source_step_seal,
};
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_spell::{
    CreatureCombatFacts, QualifiedCompanionTouches, QualifiedSpellRelocation, StagedSpellBatch,
};
pub use runtime_actor_carrier::runtime_actor_spell_types;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_spell_types::DeferredCommitAuthority;
#[allow(unused_imports)]
pub use runtime_actor_carrier::{
    ABILITY01_EFFECT_PLAN_ENTRIES_MAX, AI01_SPAWN_POPULATION_MAX, AI01_SPAWN_SOURCES_PER_SCOPE_MAX,
    ActorConditionPlan, ActorConditionTickPlan, ActorConditionTransition, ActorMemoTable,
    ApplicationFacts, AttackerCommand, AttributeModifier, AttributeModifiers,
    COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX,
    COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX, CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX,
    ChannelRuntimeV1, CombatSkill, CommittedLethalReceipt, ConditionDefinition,
    ConditionOwnerError, ConditionRefusal, ConditionSource, ConditionSourceKind, ConditionStore,
    ConditionTick, ConditionType, ConditionValues, ControlLossMark, CreatureDeathOccurrenceKey,
    CreatureDeathOccurrenceRef, CreatureSelfHealLedger, CreatureSelfHealRegistration,
    CurrentOwnerCombatDeath, CurrentOwnerExactActorCommit, CurrentOwnerExactActorLookup,
    CurrentOwnerMovementPosition, DamageSchedule, DamageSegment, DotElement, EncounterSpawnError,
    ExactSpeedRatio, FirstEntryPosition, MovementFacing, MovementLocalPosition,
    MovementPositionContext, MovementPositionSnapshot, NATIVE_SUMMON_MEMOS_PER_CHANNEL_MAX,
    NATIVE_SUMMON_RETIREMENT_LINKS_PER_CALL_MAX, NativeEncounterSpawnSpec,
    NativeSummonAdmissionSpec, NativeSummonDeath, NativeSummonMovementOutcome, NativeSummonOrigin,
    OwnerDamageCommand, OwnerDamageResult, PlayerActorReservation, RationalSpeedRange,
    RuntimeCorpseProjection, SpeedRange, StatusKind, TickFacts, TickKind,
};
#[cfg(any(test, feature = "test-support"))]
#[allow(unused_imports)] // Each path-included Foundation test crate uses only some fixtures.
pub use runtime_actor_carrier::{CombatDeathFixture, MovementActorFixture};
pub use runtime_actor_carrier::{SpellRelocationProof, relocation_seal};

#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_periodic::{
    CreaturePeriodicReceipt, PreparedCreaturePeriodicTurn,
};

pub use runtime_actor_carrier::runtime_actor_familiar_defense::FamiliarDefenseReceipt;
pub use runtime_actor_carrier::runtime_actor_party::{
    PartyAction, PartyMemberSnapshot, PartyPresence, PartySnapshot,
};
pub use runtime_actor_carrier::runtime_actor_source_reservation::{
    SourceActorReservation, SourceActorReservationProof, source_reservation_seal,
};

pub use runtime_actor_carrier::project_fixed_one_creature_death;
