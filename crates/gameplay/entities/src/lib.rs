//! L2 kernel: actor carrier and component registry with deterministic ordering.
//!
//! The physical Channel actor carrier composes the Foundation kernel and qualified Content.

#[allow(unused_imports)]
use oteryn_gameplay_domain::domain;
#[allow(unused_imports)]
pub use oteryn_gameplay_foundation::foundation::*;

pub use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
pub use oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicies;
pub use oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicy;
pub use oteryn_gameplay_content::content::creature_policy::CreatureExactRatio;
pub use oteryn_gameplay_content::content::creature_policy::CreatureFlags;
pub use oteryn_gameplay_content::content::creature_policy::CreatureResistance;
pub use oteryn_gameplay_content::content::creature_policy::FamiliarSelfHealDefense;
pub use oteryn_gameplay_domain::domain::carrier_values::CarrierError;
pub use oteryn_gameplay_domain::domain::carrier_values::LocalPosition;

pub mod content_adapters;

#[allow(dead_code)]
pub mod carrier;
pub use carrier as runtime_actor_carrier;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ABILITY01_EFFECT_PLAN_ENTRIES_MAX;
#[allow(unused_imports)]
pub use runtime_actor_carrier::AI01_SPAWN_POPULATION_MAX;
#[allow(unused_imports)]
pub use runtime_actor_carrier::AI01_SPAWN_SOURCES_PER_SCOPE_MAX;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ActorConditionPlan;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ActorConditionTickPlan;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ActorConditionTransition;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ActorMemoTable;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ApplicationFacts;
#[allow(unused_imports)]
pub use runtime_actor_carrier::AttackerCommand;
#[allow(unused_imports)]
pub use runtime_actor_carrier::AttributeModifier;
#[allow(unused_imports)]
pub use runtime_actor_carrier::AttributeModifiers;
#[allow(unused_imports)]
pub use runtime_actor_carrier::COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX;
#[allow(unused_imports)]
pub use runtime_actor_carrier::COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX;
#[allow(unused_imports)]
pub use runtime_actor_carrier::CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ChannelRuntimeV1;
#[cfg(any(test, feature = "test-support"))]
#[allow(unused_imports)] // Each path-included Foundation test crate uses only some fixtures.
pub use runtime_actor_carrier::CombatDeathFixture;
#[allow(unused_imports)]
pub use runtime_actor_carrier::CombatSkill;
#[allow(unused_imports)]
pub use runtime_actor_carrier::CommittedLethalReceipt;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ConditionDefinition;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ConditionOwnerError;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ConditionRefusal;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ConditionSource;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ConditionSourceKind;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ConditionStore;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ConditionTick;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ConditionType;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ConditionValues;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ControlLossMark;
#[allow(unused_imports)]
pub use runtime_actor_carrier::CreatureDeathOccurrenceKey;
#[allow(unused_imports)]
pub use runtime_actor_carrier::CreatureDeathOccurrenceRef;
#[allow(unused_imports)]
pub use runtime_actor_carrier::CreatureSelfHealLedger;
#[allow(unused_imports)]
pub use runtime_actor_carrier::CreatureSelfHealRegistration;
#[allow(unused_imports)]
pub use runtime_actor_carrier::CurrentOwnerCombatDeath;
#[allow(unused_imports)]
pub use runtime_actor_carrier::CurrentOwnerExactActorCommit;
#[allow(unused_imports)]
pub use runtime_actor_carrier::CurrentOwnerExactActorLookup;
#[allow(unused_imports)]
pub use runtime_actor_carrier::CurrentOwnerMovementPosition;
#[allow(unused_imports)]
pub use runtime_actor_carrier::DamageSchedule;
#[allow(unused_imports)]
pub use runtime_actor_carrier::DamageSegment;
#[allow(unused_imports)]
pub use runtime_actor_carrier::DotElement;
#[allow(unused_imports)]
pub use runtime_actor_carrier::EncounterSpawnError;
#[allow(unused_imports)]
pub use runtime_actor_carrier::ExactSpeedRatio;
#[allow(unused_imports)]
pub use runtime_actor_carrier::FirstEntryPosition;
#[cfg(any(test, feature = "test-support"))]
#[allow(unused_imports)] // Each path-included Foundation test crate uses only some fixtures.
pub use runtime_actor_carrier::MovementActorFixture;
#[allow(unused_imports)]
pub use runtime_actor_carrier::MovementFacing;
#[allow(unused_imports)]
pub use runtime_actor_carrier::MovementLocalPosition;
#[allow(unused_imports)]
pub use runtime_actor_carrier::MovementPositionContext;
#[allow(unused_imports)]
pub use runtime_actor_carrier::MovementPositionSnapshot;
#[allow(unused_imports)]
pub use runtime_actor_carrier::NATIVE_SUMMON_MEMOS_PER_CHANNEL_MAX;
#[allow(unused_imports)]
pub use runtime_actor_carrier::NATIVE_SUMMON_RETIREMENT_LINKS_PER_CALL_MAX;
#[allow(unused_imports)]
pub use runtime_actor_carrier::NativeEncounterSpawnSpec;
#[allow(unused_imports)]
pub use runtime_actor_carrier::NativeSummonAdmissionSpec;
#[allow(unused_imports)]
pub use runtime_actor_carrier::NativeSummonDeath;
#[allow(unused_imports)]
pub use runtime_actor_carrier::NativeSummonMovementOutcome;
#[allow(unused_imports)]
pub use runtime_actor_carrier::NativeSummonOrigin;
#[allow(unused_imports)]
pub use runtime_actor_carrier::OwnerDamageCommand;
#[allow(unused_imports)]
pub use runtime_actor_carrier::OwnerDamageResult;
#[allow(unused_imports)]
pub use runtime_actor_carrier::PlayerActorReservation;
#[allow(unused_imports)]
pub use runtime_actor_carrier::RationalSpeedRange;
#[allow(unused_imports)]
pub use runtime_actor_carrier::RewardDamageShare;
#[allow(unused_imports)]
pub use runtime_actor_carrier::RuntimeCorpseProjection;
#[allow(unused_imports)]
pub use runtime_actor_carrier::SpeedRange;
pub use runtime_actor_carrier::SpellRelocationProof;
#[allow(unused_imports)]
pub use runtime_actor_carrier::StatusKind;
#[allow(unused_imports)]
pub use runtime_actor_carrier::TickFacts;
#[allow(unused_imports)]
pub use runtime_actor_carrier::TickKind;
#[allow(unused_imports)]
pub use runtime_actor_carrier::VisibleRuntimeCorpse;
pub use runtime_actor_carrier::npc_actor::NpcPlacement;
pub use runtime_actor_carrier::npc_actor::NpcPlacementError;
pub use runtime_actor_carrier::npc_actor::NpcPlacementTable;
pub use runtime_actor_carrier::npc_actor::NpcPlacements;
pub use runtime_actor_carrier::project_fixed_one_creature_death;
pub use runtime_actor_carrier::relocation_seal;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_companion::CompanionMaster;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_companion::CompanionSnapshot;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_companion::CompanionState;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_companion::FamiliarDefenseClock;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_companion::PreparedCompanionAssignment;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_companion::PreparedCompanionSpawn;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_companion::PreparedCreatureProfileTransformation;
pub use runtime_actor_carrier::runtime_actor_familiar_defense::FamiliarDefenseReceipt;
pub use runtime_actor_carrier::runtime_actor_party::PartyAction;
pub use runtime_actor_carrier::runtime_actor_party::PartyMemberSnapshot;
pub use runtime_actor_carrier::runtime_actor_party::PartyPresence;
pub use runtime_actor_carrier::runtime_actor_party::PartySnapshot;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_periodic::CreaturePeriodicReceipt;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_periodic::PreparedCreaturePeriodicTurn;
pub use runtime_actor_carrier::runtime_actor_source_reservation::SourceActorReservation;
pub use runtime_actor_carrier::runtime_actor_source_reservation::SourceActorReservationProof;
pub use runtime_actor_carrier::runtime_actor_source_reservation::source_reservation_seal;
pub use runtime_actor_carrier::runtime_actor_source_step::SourceStepCommitProof;
pub use runtime_actor_carrier::runtime_actor_source_step::source_step_seal;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_spell::CreatureCombatFacts;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_spell::QualifiedCompanionTouches;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_spell::QualifiedSpellRelocation;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_spell::StagedSpellBatch;
pub use runtime_actor_carrier::runtime_actor_spell_types;
#[allow(unused_imports)]
pub use runtime_actor_carrier::runtime_actor_spell_types::DeferredCommitAuthority;
