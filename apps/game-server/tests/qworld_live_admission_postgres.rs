//! Focused execution of the production M03 owner case without unrelated authority cases.
#![allow(dead_code, unused_imports)]
extern crate oteryn_game_server as production_server;
pub use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
pub use oteryn_gameplay_dispatch::gameplay_transport;
use oteryn_gameplay_durability::durability;
pub use oteryn_gameplay_entities as foundation;
pub use production_server::{admission_evidence, character_bootstrap_intent, domain};
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[path = "support/authority_cases/character_promotion_fixture.rs"]
mod character_promotion_fixture;
#[path = "support/qworld_live_admission_cases.rs"]
mod qm03_world_admission;
