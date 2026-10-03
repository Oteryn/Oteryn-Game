#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17.6 qualification for ACHIEVEMENT step 3 account
// facts (migration 0021). Ordinary workspace runs report PRE-ROUTING/NONCANONICAL
// when the routed database is absent.
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
// The runtime Achievement catalogue of the display read, with the narrow
// `content` shim it names (as `character_authority_postgres` includes them).
#[allow(dead_code, unused_imports)]
#[path = "../src/achievement_catalogue.rs"]
pub mod achievement_catalogue;
#[allow(dead_code, unused_imports)]
#[path = "support/content_shim.rs"]
pub mod content;

#[allow(dead_code)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
// Standalone target for local focused runs; the cases need only the shared
// CHARM-2 harness above, so the protected PostgreSQL lane includes the same
// cases file.
#[path = "support/account_achievement_postgres_cases.rs"]
mod account_achievement_postgres_cases;
