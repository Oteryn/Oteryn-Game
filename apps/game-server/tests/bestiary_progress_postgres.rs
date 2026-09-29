#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17.6 qualification for CHARM-2 Bestiary kill progress
// (migration 0018). Ordinary workspace runs report PRE-ROUTING/NONCANONICAL
// when the routed database is absent.
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

#[allow(dead_code)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
// Standalone target for local focused runs; the cases need only the harness
// above, so the protected PostgreSQL lane can include the same two files.
#[path = "support/bestiary_progress_postgres_cases.rs"]
mod bestiary_progress_postgres_cases;
