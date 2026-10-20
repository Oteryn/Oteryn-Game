#![cfg(any())] // CP slot: enable with native domain/durability Guild module registration.
//! Full actual native crate-root module context for the private Premium coordinator and Root.
//! Each declaration below loads its original native physical source. No authority facade,
//! helper visibility widening, duplicated Premium owner, or production registration is added.
//! Qualification records the exact root-declaration/path provenance separately.
extern crate self as oteryn_game_server;

#[cfg(test)]
#[path = "../src/monster_lab.rs"]
mod monster_lab;

#[allow(dead_code)]
#[allow(clippy::duplicate_mod)] // Foundation's standalone tests include Ability source.
#[path = "../src/ability/mod.rs"]
mod ability;
#[path = "../src/achievement_catalogue.rs"]
mod achievement_catalogue;
#[allow(
    dead_code,
    unused_imports,
    reason = "ai:: has no production caller yet; a later wiring task consumes it from ai_think"
)]
#[path = "../src/ai/mod.rs"]
mod ai;
#[path = "../src/ai_monster_melee.rs"]
mod ai_monster_melee;
// Ordinary declarations retain the native non-mod.rs implicit child directories. A direct
// #[path="../src/ai_think.rs"] would instead search src/ for its implicit children.
#[path = "../src"]
mod native_file_modules {
    #[allow(
        dead_code,
        reason = "ai_think has no production caller yet; AI-4/a later wiring task composes it into ChannelRuntimeV1's owner cycle"
    )]
    pub(crate) mod ai_think;
    #[allow(
        dead_code,
        reason = "unactivated local Movement proof awaits the separate production composition lease"
    )]
    pub(crate) mod movement;
}
use native_file_modules::{ai_think, movement};
#[allow(
    dead_code,
    reason = "nonshipping fixed-one-creature Combat proof awaits production composition"
)]
#[path = "../src/combat.rs"]
mod combat;
#[path = "../src/combat/attack/mod.rs"]
mod combat_attack;
#[allow(
    dead_code,
    reason = "B3-2 has no production caller yet; a later admission stage wires one"
)]
#[path = "../src/combat/pickup.rs"]
mod combat_pickup;
#[allow(
    dead_code,
    reason = "PvP stays off until the PVP-0 go-live gate; PVP-RT-1b wires the legality stage"
)]
#[path = "../src/combat/pvp/mod.rs"]
mod combat_pvp;
#[path = "../src/interaction/chest_use.rs"]
mod interaction_chest_use;
#[allow(dead_code)] // Preserve the original Ability module lint scope for this native-only adapter.
#[path = "../src/ability/player_lethal.rs"]
mod player_lethal;

#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
pub use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
#[allow(
    dead_code,
    unused_imports,
    reason = "CHAT-1a local chat rules await the CHAT_V1 wire and runtime of CHAT-1b"
)]
#[path = "../src/chat/mod.rs"]
mod chat;
#[path = "../src/content/mod.rs"]
pub mod content;
#[path = "../../../crates/gameplay/domain/src/domain/mod.rs"]
pub mod domain;
#[path = "../src/durability/mod.rs"]
pub mod durability;
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[path = "../src/gameplay_transport/mod.rs"]
mod gameplay_transport;

pub use gameplay_transport::{
    FreshEvidenceSource, GameplayListenerConfig, GameplaySeamOwners, GameplayServeError,
    serve_gameplay,
};
#[path = "../src/house_scope/mod.rs"]
pub mod house_scope;
#[path = "../src/map/mod.rs"]
pub mod map;

#[path = "../src/node/mod.rs"]
pub mod node;
#[path = "../src/premium/mod.rs"]
pub mod premium;
#[allow(
    dead_code,
    reason = "player spell core (P3a) awaits the cast protocol and runtime state contracts (P3b)"
)]
#[path = "../src/spell/mod.rs"]
mod spell;
#[path = "../src/wheel_gem_data.rs"]
pub mod wheel_gem_data;

#[allow(
    dead_code,
    unused_imports,
    reason = "GAME-INTERACTION identity; the §7 revert driver is its first library consumer"
)]
#[path = "../src/interaction/mod.rs"]
mod interaction;
#[allow(dead_code)] // Preserve the original Spell adapter lint scope.
#[path = "../src/durability/native_world_items.rs"]
mod native_world_items;
#[allow(dead_code)] // Preserve the original Spell adapter lint scope.
#[path = "../src/durability/native_world_read.rs"]
mod native_world_read;
#[allow(
    dead_code,
    reason = "§7 timed-revert driver awaits its live Channel/Instance scope owner"
)]
#[path = "../src/world_object_revert.rs"]
mod world_object_revert;
#[allow(dead_code)]
#[path = "../src/world_runtime.rs"]
pub(crate) mod world_runtime;

#[cfg(test)]
#[allow(
    dead_code,
    reason = "shared PostgreSQL support; each mount uses a subset"
)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

#[cfg(test)]
#[path = "support/spell_stance_runtime_postgres_cases.rs"]
mod spell_stance_runtime_postgres_cases;

#[cfg(test)]
#[path = "support/spell_familiar_runtime_postgres_cases.rs"]
mod spell_familiar_runtime_postgres_cases;

#[cfg(test)]
#[path = "../src/durability/world_party_tests.rs"]
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
#[path = "../src/bone_phase_registry.rs"]
mod bone_phase_registry;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_appearance_content.rs"]
mod creature_appearance_content;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_area_heal.rs"]
mod creature_area_heal;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_attack_geometry.rs"]
mod creature_attack_geometry;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_auto_attack.rs"]
mod creature_auto_attack;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_chain_attack.rs"]
mod creature_chain_attack;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_condition_content.rs"]
mod creature_condition_content;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_damage_spell.rs"]
mod creature_damage_spell;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/creature_defense_presentation.rs"]
mod creature_defense_presentation;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/crystal_callbacks.rs"]
mod crystal_callbacks;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/crystal_death_composition.rs"]
mod crystal_death_composition;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/crystal_herald_quest.rs"]
mod crystal_herald_quest;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/self_heal_qualification.rs"]
mod self_heal_qualification;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/smelly_cheese.rs"]
mod smelly_cheese;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/smelly_damage.rs"]
mod smelly_damage;
#[allow(dead_code)]
#[path = "../src/smelly_encounter_dispatch.rs"]
mod smelly_encounter_dispatch;
#[path = "../src/source_callback_intent.rs"]
mod source_callback_intent;
#[allow(dead_code)]
#[path = "../src/source_callback_spawn.rs"]
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
#[path = "../src/source_encounter_seven.rs"]
mod source_encounter_seven;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/source_threshold_heal.rs"]
mod source_threshold_heal;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/weak_spot_speech.rs"]
mod weak_spot_speech;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/welter_consume.rs"]
mod welter_consume;
#[allow(
    dead_code,
    reason = "LOOP-CLOCK-1 registers the pure World clock; the WORLD_LIGHT_V1 offer composes it later"
)]
#[path = "../src/world_clock.rs"]
mod world_clock;

#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/monster_combat_lane.rs"]
mod monster_combat_lane;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/monster_owner_cycle.rs"]
mod monster_owner_cycle;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[path = "../src/monster_summon.rs"]
mod monster_summon;

#[cfg(test)]
#[path = "../src/canonical_monster_successor_tests.rs"]
mod canonical_monster_successor_tests;

#[allow(dead_code)]
#[path = "../src/creature_icicle.rs"]
mod creature_icicle;

#[path = "../src/durability/creature_source_items.rs"]
mod creature_source_items;

#[allow(
    dead_code,
    unused_imports,
    clippy::expect_used,
    reason = "literal shared PG fixture uses the original native test expectation context"
)]
#[path = "support/item_transfer_postgres_cases.rs"]
mod item_transfer_postgres_cases;
