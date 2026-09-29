#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17.6 qualification for the CHARM-2 Bestiary descendant
// of a committed creature death (`combat/death_reward.rs`). Ordinary
// workspace runs report PRE-ROUTING/NONCANONICAL when the routed database is
// absent.
extern crate self as oteryn_game_server;

#[allow(dead_code, unused_imports)]
#[path = "../src/admission_evidence.rs"]
pub mod admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../src/domain/mod.rs"]
pub mod domain;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;

// `combat.rs` re-exports only the pre-CHARM-2 composition, so this target
// loads the two Combat reward modules directly as crate-root siblings (their
// `super::loot_plan` then resolves to the module below).
#[allow(dead_code, unused_imports)]
#[path = "../src/combat/death_reward.rs"]
mod death_reward;
#[allow(dead_code, unused_imports)]
#[path = "../src/combat/loot_plan.rs"]
mod loot_plan;

#[allow(dead_code)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[path = "support/combat_bestiary_postgres_cases.rs"]
mod combat_bestiary_postgres_cases;
