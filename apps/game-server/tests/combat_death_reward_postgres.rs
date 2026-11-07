#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 18 qualification for Combat D2b (creature death ->
// loot MINT + R7 P03 XP composition). Ordinary workspace runs report
// PRE-ROUTING/NONCANONICAL when the routed database is absent.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;

pub use production_server::admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../../../crates/gameplay/durability/src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../../../crates/gameplay/combat/src/combat/mod.rs"]
pub mod combat;
#[path = "support/domain_shim.rs"]
pub mod domain;
use oteryn_gameplay_durability::durability;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_entities as foundation;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_native_admission_source as native_admission_source;

// Shared PostgreSQL harness for this target's integration cases.
#[allow(dead_code)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

#[path = "support/attack_kill_reward_postgres_cases.rs"]
mod attack_kill_reward_postgres_cases;
#[path = "support/combat_death_reward_postgres_cases.rs"]
mod combat_death_reward_postgres_cases;

#[allow(dead_code, unused_imports)]
#[path = "support/content_spell_data_shim.rs"]
mod content;
