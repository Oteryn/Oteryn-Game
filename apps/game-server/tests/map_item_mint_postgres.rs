#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17 qualification for MAP-OVERLAY-1b (DUR-03 map-item
// MINT of a map-authored entry into Ground at its tile, then the B3-1
// TRANSFER), plus the overlay side of the pickup against a fixture bundle;
// the cases live in support/map_item_mint_postgres_cases.rs.
// Ordinary workspace runs report PRE-ROUTING/NONCANONICAL when the routed
// database is absent.
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

// Standalone target for local focused runs; the protected PostgreSQL lane also
// includes these exact cases through character_authority_postgres.
#[path = "support/map_item_mint_postgres_cases.rs"]
mod map_item_mint_postgres_cases;
