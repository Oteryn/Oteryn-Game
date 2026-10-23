#![allow(clippy::expect_used)]
// D1112 APP-1 native authority target. Every case runs against the routed PostgreSQL 18.6
// service and the installed migration 0093; without the service it skips as PRE-ROUTING.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../../../crates/gameplay/durability/src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
#[path = "support/domain_shim.rs"]
pub mod domain;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability"]
mod durability {
    // Recompile the shared core; native world adapters use the full library Content context.
    include!("../../../crates/gameplay/durability/src/durability/core_modules.rs");
}
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_entities as foundation;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_native_admission_source as native_admission_source;
#[allow(dead_code, unused_imports)]
#[path = "../../../crates/gameplay/social/src/premium/mod.rs"]
mod premium;

// Actual registered native owner and installed migration 0093. Qualification uses
// the committed head directly, with no source overlay or supplemental SQL.
#[allow(
    dead_code,
    reason = "shared native harness includes helpers for other receipt families"
)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
// Standalone target for local focused runs; CI runs the same cases through
// `character_authority_postgres`.
#[path = "support/character_appearance_native_postgres_cases.rs"]
mod character_appearance_native_postgres_cases;

#[allow(dead_code, unused_imports)]
#[path = "support/content_spell_data_shim.rs"]
mod content;
