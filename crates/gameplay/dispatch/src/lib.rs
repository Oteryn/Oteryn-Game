//! ADR-0019 gameplay crate `oteryn-gameplay-dispatch`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

pub mod gameplay_transport;

#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_admission_evidence as admission_evidence;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::ability;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::combat;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::combat_attack;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::combat_pickup;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::combat_pvp;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::creature_appearance_content;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::creature_area_heal;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::creature_attack_geometry;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::creature_auto_attack;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::creature_chain_attack;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::creature_condition_content;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::creature_defense_presentation;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::creature_icicle;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::creature_spell_facts;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_combat::player_lethal;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_content::content;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_domain::domain;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_durability::character_bootstrap_intent;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_durability::durability;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_encounters::crystal_callbacks;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_encounters::crystal_death_composition;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_encounters::crystal_herald_quest;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_encounters::self_heal_qualification;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_encounters::source_callback_cast;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_encounters::source_callback_intent;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_encounters::source_callback_spawn;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_encounters::source_encounter_seven;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_encounters::source_threshold_heal;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_foundation::foundation;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_monster_ai::ai;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_monster_ai::ai_monster_melee;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_monster_ai::ai_think;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_monster_ai::bone_phase_registry;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_monster_ai::monster_combat_lane;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_monster_ai::monster_owner_cycle;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_monster_ai::monster_summon;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_monster_ai::smelly_cheese;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_monster_ai::smelly_damage;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_monster_ai::smelly_encounter_dispatch;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_monster_ai::weak_spot_speech;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_native_admission_source as native_admission_source;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_native_admission_source::account_characters;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_native_admission_source::character_commands;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_native_admission_source::descriptor;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_native_admission_source::http1_mtls;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_native_admission_source::public_projections;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_native_admission_source::runtime_status;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_native_admission_source::scope_assignment;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_node_files::operator_files;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_node_files::secure_file;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_social::achievement_catalogue;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_social::chat;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_social::interaction;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_social::interaction_chest_use;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_social::party;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_social::premium;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_social::security;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_social::wheel_gem_data;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_spell::creature_damage_spell;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_spell::creature_damage_spell_delayed;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_spell::spell;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_world::house_scope;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_world::map;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_world::movement;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_world::world_clock;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_world::world_object_revert;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_world::world_runtime;

pub use gameplay_transport::{GameplayListenerConfig, GameplaySeamOwners, serve_gameplay};
pub(crate) use oteryn_gameplay_weather as world_weather;

#[cfg(test)]
#[path = "lifted_tests/durability_world_party_tests.rs"]
mod world_party_runtime_postgres_cases;

#[cfg(test)]
#[path = "lifted_tests/content_npc_service_tests.rs"]
mod content_npc_service_tests;

#[cfg(test)]
#[path = "lifted_tests/movement_speed_runtime_tests.rs"]
mod movement_speed_runtime_tests;

#[cfg(test)]
#[path = "lifted_tests/creature_area_heal_tests.rs"]
mod creature_area_heal_tests;

#[cfg(test)]
#[path = "lifted_tests/creature_auto_attack_tests.rs"]
mod creature_auto_attack_tests;

#[cfg(test)]
#[path = "lifted_tests/creature_defense_presentation_tests.rs"]
mod creature_defense_presentation_tests;

#[cfg(test)]
#[path = "lifted_tests/creature_damage_spell_tests.rs"]
mod creature_damage_spell_tests;

#[cfg(test)]
#[path = "lifted_tests/creature_damage_spell_defense_variant_tests.rs"]
mod creature_damage_spell_defense_variant_tests;

#[cfg(test)]
#[path = "lifted_tests/creature_damage_spell_appearance_execution_tests.rs"]
mod creature_damage_spell_appearance_execution_tests;

#[cfg(test)]
#[path = "lifted_tests/creature_damage_spell_composed_tests.rs"]
mod creature_damage_spell_composed_tests;

#[cfg(test)]
#[path = "lifted_tests/spell_house_execution_tests.rs"]
mod spell_house_execution_tests;

#[cfg(test)]
#[path = "lifted_tests/spell_owned_cast_facts_tests.rs"]
mod spell_owned_cast_facts_tests;

#[cfg(test)]
#[path = "lifted_tests/spell_field_history_tests.rs"]
mod spell_field_history_tests;

#[cfg(test)]
#[path = "lifted_tests/spell_companion_lifecycle_direct_acquisition_owner_tests.rs"]
mod spell_companion_lifecycle_direct_acquisition_owner_tests;

#[cfg(test)]
#[path = "lifted_tests/spell_actor_execution_tests.rs"]
mod spell_actor_execution_tests;

#[cfg(test)]
#[path = "lifted_tests/spell_magnitude_owner_tests.rs"]
mod spell_magnitude_owner_tests;

#[cfg(test)]
#[path = "lifted_tests/spell_native_cast_commit_tests.rs"]
mod spell_native_cast_commit_tests;

#[cfg(test)]
#[path = "lifted_tests/spell_delayed_execution_tests.rs"]
mod spell_delayed_execution_tests;

#[cfg(test)]
#[path = "lifted_tests/spell_callback_speech_tests.rs"]
mod spell_callback_speech_tests;

#[cfg(test)]
#[path = "lifted_tests/source_callback_cast_tests.rs"]
mod source_callback_cast_tests;

#[cfg(test)]
#[path = "lifted_tests/source_callback_spawn_tests.rs"]
mod source_callback_spawn_tests;

#[cfg(test)]
#[path = "lifted_tests/source_encounter_seven_tests.rs"]
mod source_encounter_seven_tests;

#[cfg(test)]
#[path = "lifted_tests/source_threshold_heal_tests.rs"]
mod source_threshold_heal_tests;

#[cfg(test)]
#[path = "lifted_tests/monster_owner_cycle_tests.rs"]
mod monster_owner_cycle_tests;

#[cfg(test)]
#[path = "lifted_tests/smelly_damage_tests.rs"]
mod smelly_damage_tests;

#[cfg(test)]
#[path = "lifted_tests/smelly_encounter_dispatch_tests.rs"]
mod smelly_encounter_dispatch_tests;

#[cfg(test)]
#[path = "lifted_tests/monster_combat_lane_tests.rs"]
mod monster_combat_lane_tests;

#[cfg(test)]
#[path = "lifted_tests/monster_combat_lane_defense_composition_tests.rs"]
mod monster_combat_lane_defense_composition_tests;

#[cfg(test)]
#[path = "lifted_tests/monster_combat_lane_defense_speed_test.rs"]
mod monster_combat_lane_defense_speed_test;

#[cfg(test)]
#[path = "lifted_tests/monster_combat_lane_source_summon_composition_tests.rs"]
mod monster_combat_lane_source_summon_composition_tests;

#[allow(dead_code)]
pub mod welter_consume;

#[cfg(test)]
mod canonical_monster_successor_tests;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

#[cfg(test)]
#[path = "lifted_tests/source_time_profile_bundle_tests.rs"]
mod source_time_profile_bundle_tests;
