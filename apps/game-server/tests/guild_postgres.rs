//! GUILD-1 core through the production crate graph and the existing native PG harness.
#![allow(clippy::expect_used)]
extern crate self as oteryn_game_server;
pub use oteryn_game_server_library::{
    admission_evidence, character_bootstrap_intent, character_recovery_fence, content, domain,
    durability, foundation, native_admission_source,
};
extern crate oteryn_game_server as oteryn_game_server_library;
use oteryn_gameplay_dispatch::gameplay_transport;
#[path = "../src/lifted_tests/durability_guild_tests.rs"]
mod durability_guild_tests;
#[allow(dead_code)]
#[path = "support/item_transfer_postgres_harness.rs"]
mod item_transfer_postgres_cases;
#[path = "support/guild_premium_test_source.rs"]
pub mod premium;
