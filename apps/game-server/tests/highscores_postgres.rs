#![cfg(any())] // CP: enable with native module and canonical aggregate registration.
//! Genuine private-root target; inactive before CP registration, not a zero-test runtime pass.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::admission_evidence;
pub use production_server::content;
#[allow(dead_code, unused_imports)]
#[path = "../src/achievement_catalogue.rs"]
mod achievement_catalogue;
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
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;
#[allow(dead_code, unused_imports)]
#[path = "../src/premium/mod.rs"]
mod premium;
