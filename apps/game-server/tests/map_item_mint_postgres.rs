#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 18 qualification for MAP-OVERLAY-1b (DUR-03 map-item
// MINT of a map-authored entry into Ground at its tile, then the B3-1
// TRANSFER), plus the overlay side of the pickup against a fixture bundle;
// the cases live in support/map_item_mint_postgres_cases.rs.
// Ordinary workspace runs report PRE-ROUTING/NONCANONICAL when the routed
// database is absent.
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
use oteryn_gameplay_durability::durability;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_entities as foundation;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_native_admission_source as native_admission_source;

// Standalone target for local focused runs; the protected PostgreSQL lane also
// includes these exact cases through character_authority_postgres.
#[path = "support/map_item_mint_postgres_cases.rs"]
mod map_item_mint_postgres_cases;

#[allow(dead_code, unused_imports)]
#[path = "support/content_spell_data_shim.rs"]
mod content;

// Shared PostgreSQL harness for this target's integration cases.
#[allow(dead_code)] // Each standalone target uses only part of the shared fixture API.
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
