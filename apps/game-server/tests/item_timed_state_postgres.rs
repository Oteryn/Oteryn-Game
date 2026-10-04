#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17.6 qualification for TIMED-RT-1a (migration 0054). Ordinary workspace runs
// report PRE-ROUTING/NONCANONICAL when the routed database is absent.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;

pub use production_server::admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
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

#[allow(dead_code)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
// Standalone target for local focused runs. The cases need only the path-loaded crate root and
// the harness above, so a protected PostgreSQL wrapper can include the same two files.
#[path = "support/item_timed_state_postgres_cases.rs"]
mod item_timed_state_postgres_cases;
