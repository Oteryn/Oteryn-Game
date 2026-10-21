#![cfg(any())] // CP slot: enable with native domain/durability Guild module registration.
//! Full actual native crate-root module context for the private Premium coordinator and Root.
//! Each declaration below loads its original native physical source. No authority facade,
//! helper visibility widening, duplicated Premium owner, or production registration is added.
//! Qualification records the exact root-declaration/path provenance separately.
extern crate self as oteryn_game_server;

#[cfg(test)]
#[path = "../../../crates/gameplay/monster-ai/src/monster_lab.rs"]
mod monster_lab;

#[allow(dead_code)]
#[allow(clippy::duplicate_mod)] // Foundation's standalone tests include Ability source.
#[path = "../../../crates/gameplay/combat/src/ability/mod.rs"]
mod ability;
#[path = "../../../crates/gameplay/social/src/achievement_catalogue.rs"]
mod achievement_catalogue;
#[allow(
    dead_code,
    unused_imports,
    reason = "ai:: has no production caller yet; a later wiring task consumes it from ai_think"
)]
#[path = "../../../crates/gameplay/monster-ai/src/ai/mod.rs"]
mod ai;
#[path = "../../../crates/gameplay/monster-ai/src/ai_monster_melee.rs"]
mod ai_monster_melee;
use oteryn_gameplay_monster_ai::{ai_think, monster_combat_lane};
use oteryn_gameplay_world::movement;
#[allow(
    dead_code,
    reason = "nonshipping fixed-one-creature Combat proof awaits production composition"
)]
#[path = "../../../crates/gameplay/combat/src/combat/mod.rs"]
mod combat;
#[path = "../../../crates/gameplay/combat/src/combat/attack/mod.rs"]
mod combat_attack;
#[allow(
    dead_code,
    reason = "B3-2 has no production caller yet; a later admission stage wires one"
)]
#[path = "../../../crates/gameplay/combat/src/combat/pickup.rs"]
mod combat_pickup;
#[allow(
    dead_code,
    reason = "PvP stays off until the PVP-0 go-live gate; PVP-RT-1b wires the legality stage"
)]
#[path = "../../../crates/gameplay/combat/src/combat/pvp/mod.rs"]
mod combat_pvp;
#[path = "../../../crates/gameplay/social/src/interaction/chest_use.rs"]
mod interaction_chest_use;
#[allow(dead_code)] // Preserve the original Ability module lint scope for this native-only adapter.
#[path = "../../../crates/gameplay/combat/src/ability/player_lethal.rs"]
mod player_lethal;

#[path = "../../../crates/gameplay/durability/src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
pub use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
#[allow(
    dead_code,
    unused_imports,
    reason = "CHAT-1a local chat rules await the CHAT_V1 wire and runtime of CHAT-1b"
)]
#[path = "../../../crates/gameplay/social/src/chat/mod.rs"]
mod chat;
#[path = "../../../crates/gameplay/content/src/content/mod.rs"]
pub mod content;
#[path = "../../../crates/gameplay/domain/src/domain/mod.rs"]
pub mod domain;
#[path = "../../../crates/gameplay/durability/src/durability/mod.rs"]
pub mod durability;
#[path = "../../../crates/gameplay/foundation/src/foundation/mod.rs"]
pub mod foundation;
#[path = "../../../crates/gameplay/dispatch/src/gameplay_transport/mod.rs"]
mod gameplay_transport;

pub use gameplay_transport::{
    FreshEvidenceSource, GameplayListenerConfig, GameplaySeamOwners, GameplayServeError,
    serve_gameplay,
};
#[path = "../../../crates/gameplay/world/src/house_scope/mod.rs"]
pub mod house_scope;
#[path = "../../../crates/gameplay/world/src/map/mod.rs"]
pub mod map;

#[path = "../src/node/mod.rs"]
pub mod node;
#[path = "../../../crates/gameplay/social/src/premium/mod.rs"]
pub mod premium;
#[allow(
    dead_code,
    reason = "player spell core (P3a) awaits the cast protocol and runtime state contracts (P3b)"
)]
#[path = "../../../crates/gameplay/spell/src/spell/mod.rs"]
mod spell;
#[path = "../../../crates/gameplay/social/src/wheel_gem_data.rs"]
pub mod wheel_gem_data;

#[allow(
    dead_code,
    unused_imports,
    reason = "GAME-INTERACTION identity; the §7 revert driver is its first library consumer"
)]
#[path = "../../../crates/gameplay/social/src/interaction/mod.rs"]
mod interaction;
#[allow(dead_code)] // Preserve the original Spell adapter lint scope.
use durability::{native_world_items, native_world_read};
#[allow(
    dead_code,
    reason = "§7 timed-revert driver awaits its live Channel/Instance scope owner"
)]
#[path = "../../../crates/gameplay/world/src/world_object_revert.rs"]
mod world_object_revert;
#[allow(dead_code)]
#[path = "../../../crates/gameplay/world/src/world_runtime.rs"]
pub(crate) mod world_runtime;

#[cfg(test)]
#[allow(
    dead_code,
    reason = "shared PostgreSQL support; each mount uses a subset"
)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

// Spell Stance/Familiar PostgreSQL cases are registered by src/lib.rs, where their native
// Spell owner is available without importing it into neutral durability test roots.

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/durability_world_party_tests.rs"]
mod world_party_runtime_postgres_cases;

#[cfg(test)]
#[path = "../src/lifted_tests/foundation_recovery_tests.rs"]
mod foundation_recovery_tests;

#[cfg(test)]
#[path = "../src/lifted_tests/foundation_final_review_regressions.rs"]
mod foundation_final_review_regressions;

#[cfg(test)]
#[path = "../src/lifted_tests/foundation_final_review_round2_regressions.rs"]
mod foundation_final_review_round2_regressions;

#[cfg(test)]
#[path = "../src/lifted_tests/foundation_final_review_round2_runtime_rollback.rs"]
mod foundation_final_review_round2_runtime_rollback;

pub use oteryn_gameplay_admission_evidence as admission_evidence;
pub use oteryn_gameplay_native_admission_source as native_admission_source;

#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/monster-ai/src/bone_phase_registry.rs"]
mod bone_phase_registry;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/combat/src/creature_appearance_content.rs"]
mod creature_appearance_content;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/combat/src/creature_area_heal.rs"]
mod creature_area_heal;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/combat/src/creature_attack_geometry.rs"]
mod creature_attack_geometry;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/combat/src/creature_auto_attack.rs"]
mod creature_auto_attack;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/combat/src/creature_chain_attack.rs"]
mod creature_chain_attack;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/combat/src/creature_condition_content.rs"]
mod creature_condition_content;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/spell/src/creature_damage_spell/mod.rs"]
mod creature_damage_spell;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/spell/src/creature_damage_spell_delayed.rs"]
mod creature_damage_spell_delayed;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/combat/src/creature_defense_presentation.rs"]
mod creature_defense_presentation;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/encounters/src/crystal_callbacks.rs"]
mod crystal_callbacks;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/encounters/src/crystal_death_composition.rs"]
mod crystal_death_composition;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/encounters/src/crystal_herald_quest.rs"]
mod crystal_herald_quest;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/encounters/src/self_heal_qualification.rs"]
mod self_heal_qualification;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/monster-ai/src/smelly_cheese.rs"]
mod smelly_cheese;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/monster-ai/src/smelly_damage.rs"]
mod smelly_damage;
#[allow(dead_code)]
#[path = "../../../crates/gameplay/monster-ai/src/smelly_encounter_dispatch.rs"]
mod smelly_encounter_dispatch;
#[path = "../../../crates/gameplay/encounters/src/source_callback_intent.rs"]
mod source_callback_intent;
#[allow(dead_code)]
#[path = "../../../crates/gameplay/encounters/src/source_callback_spawn.rs"]
mod source_callback_spawn;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
use oteryn_gameplay_source_critical_baseline as source_critical_baseline;
#[allow(
    dead_code,
    reason = "Private native encounter owner ABI retained for source-qualified library dispatch; shipping AI loop is not activated"
)]
#[path = "../../../crates/gameplay/encounters/src/source_encounter_seven.rs"]
mod source_encounter_seven;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/encounters/src/source_threshold_heal.rs"]
mod source_threshold_heal;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/monster-ai/src/weak_spot_speech.rs"]
mod weak_spot_speech;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/dispatch/src/welter_consume.rs"]
mod welter_consume;
#[allow(
    dead_code,
    reason = "LOOP-CLOCK-1 registers the pure World clock; the WORLD_LIGHT_V1 offer composes it later"
)]
#[path = "../../../crates/gameplay/world/src/world_clock.rs"]
mod world_clock;

#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/monster-ai/src/monster_owner_cycle.rs"]
mod monster_owner_cycle;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/monster-ai/src/monster_summon.rs"]
mod monster_summon;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/canonical_monster_successor_tests.rs"]
mod canonical_monster_successor_tests;

#[allow(dead_code)]
#[path = "../../../crates/gameplay/combat/src/creature_icicle.rs"]
mod creature_icicle;

#[path = "../../../crates/gameplay/durability/src/durability/creature_source_items.rs"]
mod creature_source_items;

#[allow(
    dead_code,
    unused_imports,
    clippy::expect_used,
    reason = "literal shared PG fixture uses the original native test expectation context"
)]
#[path = "support/item_transfer_postgres_cases.rs"]
mod item_transfer_postgres_cases;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/movement_speed_runtime_tests.rs"]
mod movement_speed_runtime_tests;

#[cfg(test)]
#[path = "../src/lifted_tests/world_clock_world_config_tests.rs"]
mod world_clock_world_config_tests;

#[cfg(test)]
#[path = "../src/lifted_tests/durability_guild_tests.rs"]
mod durability_guild_tests;

#[cfg(test)]
#[path = "../src/lifted_tests/durability_vip_tests.rs"]
mod durability_vip_tests;

#[cfg(test)]
#[path = "../src/lifted_tests/durability_guild_disband_terminal_tests.rs"]
mod durability_guild_disband_terminal_tests;

#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../../../crates/gameplay/encounters/src/source_callback_cast.rs"]
mod source_callback_cast;
#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/source_callback_cast_tests.rs"]
mod source_callback_cast_tests;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/source_callback_spawn_tests.rs"]
mod source_callback_spawn_tests;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/source_encounter_seven_tests.rs"]
mod source_encounter_seven_tests;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/source_threshold_heal_tests.rs"]
mod source_threshold_heal_tests;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/monster_owner_cycle_tests.rs"]
mod monster_owner_cycle_tests;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/smelly_damage_tests.rs"]
mod smelly_damage_tests;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/smelly_encounter_dispatch_tests.rs"]
mod smelly_encounter_dispatch_tests;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/monster_combat_lane_tests.rs"]
mod monster_combat_lane_tests;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/monster_combat_lane_defense_composition_tests.rs"]
mod monster_combat_lane_defense_composition_tests;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/monster_combat_lane_defense_speed_test.rs"]
mod monster_combat_lane_defense_speed_test;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/monster_combat_lane_source_summon_composition_tests.rs"]
mod monster_combat_lane_source_summon_composition_tests;

#[cfg(test)]
#[path = "../../../crates/gameplay/dispatch/src/lifted_tests/source_time_profile_bundle_tests.rs"]
mod source_time_profile_bundle_tests;
