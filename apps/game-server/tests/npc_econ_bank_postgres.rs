#![allow(clippy::expect_used)]
//! NPC Bank continuation and NPC BUY bank-part qualification through actual native Root APIs.
//! Fixtures retain every native guard and fund value only by native MINT, TRANSFER and DEPOSIT.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::admission_evidence;
pub use production_server::domain;
#[allow(dead_code, unused_imports)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
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
#[path = "support/item_transfer_postgres_cases.rs"]
mod item_transfer_postgres_cases;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;

// The protected PostgreSQL aggregate consumes these same case bodies.
#[path = "support/npc_econ_bank_postgres_cases.rs"]
mod npc_econ_bank_postgres_cases;
