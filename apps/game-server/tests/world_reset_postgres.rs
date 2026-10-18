#![allow(clippy::expect_used)]
// Native standalone wrapper; the aggregate registers the same cases (D1416).
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::admission_evidence;
#[allow(dead_code)] // the durability test modules mounted here share the native harness
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
pub use production_server::domain;
#[path = "support/corpse_transfer_postgres_cases.rs"]
mod corpse_transfer_postgres_cases;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code)] // the reset House case uses only `install`
#[path = "support/house_test_fixture.rs"]
mod house_test_fixture;
#[path = "support/item_transfer_postgres_cases.rs"]
mod item_transfer_postgres_cases;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;
#[path = "support/world_reset_postgres_cases.rs"]
mod world_reset_postgres_cases;
