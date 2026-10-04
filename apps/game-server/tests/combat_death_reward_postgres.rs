#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17 qualification for Combat D2b (creature death ->
// loot MINT + R7 P03 XP composition). Ordinary workspace runs report
// PRE-ROUTING/NONCANONICAL when the routed database is absent.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;

pub use production_server::admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../src/combat.rs"]
pub mod combat;
pub use production_server::domain;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;

#[path = "support/attack_kill_reward_postgres_cases.rs"]
mod attack_kill_reward_postgres_cases;
#[path = "support/combat_death_reward_postgres_cases.rs"]
mod combat_death_reward_postgres_cases;
