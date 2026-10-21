//! ADR-0019 gameplay crate `oteryn-gameplay-encounters`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

pub mod bosses;
pub mod crystal_callbacks;
pub mod crystal_death_composition;
pub mod crystal_herald_quest;
pub mod self_heal_qualification;
pub mod source_callback_cast;
pub mod source_callback_intent;
pub mod source_callback_spawn;
pub mod source_encounter_seven;
pub mod source_threshold_heal;

#[allow(unused_imports, reason = "preserve the former server module path")]
pub use bosses::icicle::creature_icicle;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::ability;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::combat;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_appearance_content;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_area_heal;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_attack_geometry;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_auto_attack;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_chain_attack;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_condition_content;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_combat::creature_defense_presentation;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_content::content;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_domain::domain;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_durability::character_bootstrap_intent;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_durability::durability;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) mod foundation {
    #[cfg(any(test, feature = "test-support"))]
    pub use crate::bosses::bone_overlord::native_owner::bone_shared_actual_owner_harness;
    pub use crate::bosses::bone_overlord::native_owner::{
        BoneOverlordCagePhase, BoneOverlordPhase, BonePhaseError, BonePhaseTransition,
    };
    #[cfg(any(test, feature = "test-support"))]
    pub use crate::bosses::crystal_deaths::native_fixtures::{
        crystal_death_router_fixture, crystal_death_router_fixture_with_artifact,
    };
    pub use crate::bosses::the_welter::native_owner::{
        WelterCarrierExt, WelterConsumeError, WelterConsumeLedger, WelterConsumeResult, WelterPrey,
        WelterSourceRegistration,
    };
    pub use oteryn_gameplay_entities::*;
}
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_spell::creature_damage_spell;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_spell::creature_damage_spell_delayed;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_spell::spell;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::house_scope;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::map;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::movement;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::world_clock;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::world_object_revert;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_world::world_runtime;

pub(crate) use oteryn_gameplay_combat::player_lethal;

pub(crate) use oteryn_gameplay_source_critical_baseline as source_critical_baseline;

pub use bosses::smelly_cheese::timers as smelly_cheese;

pub use bosses::smelly_cheese::smelly_damage;

pub use bosses::smelly_cheese::smelly_encounter_dispatch;

pub use bosses::bone_overlord::bone_phase_registry;

pub use bosses::weak_spot::weak_spot_speech;

pub mod monster_combat_lane;
pub(crate) use oteryn_gameplay_monster_ai::{ai_think, monster_owner_cycle, monster_summon};

pub(crate) use crate::durability::creature_source_items;
extern crate self as oteryn_gameplay_encounters;

pub use bosses::source_callbacks::callback_speech;
