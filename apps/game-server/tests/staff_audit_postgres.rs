#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Native PostgreSQL 17.6 qualification of the production participant, on its
//! real physical transaction. Missing PG is PRE-ROUTING, never a passing PG run.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::{admission_evidence, domain, node, staff};
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;

// `durability/npc_travel.rs` mounts its PG fixture under cfg(test), which needs this harness.
#[allow(dead_code, unused_imports)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

// The canonical durability target mounts these same cases.
#[path = "support/staff_audit_postgres_cases.rs"]
mod staff_audit_postgres_cases;
