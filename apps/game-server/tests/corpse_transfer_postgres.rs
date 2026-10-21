#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17 qualification for D3-4 (DUR-03 TRANSFER out of a
// corpse container: the D133 exclusivity window and the corpse never a source). Ordinary
// workspace runs report PRE-ROUTING/NONCANONICAL when the routed database is
// absent.
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

// Reuses the B3-1 harness (Database/Harness/backpack fixture) instead of
// duplicating PostgreSQL/admission bootstrap.
#[path = "support/item_transfer_postgres_cases.rs"]
mod item_transfer_postgres_cases;

// Standalone target for local focused runs; the protected PostgreSQL lane also
// includes these exact cases through character_authority_postgres.
#[path = "support/corpse_transfer_postgres_cases.rs"]
mod corpse_transfer_postgres_cases;

#[allow(dead_code, unused_imports)]
#[path = "support/content_spell_data_shim.rs"]
mod content;

// The path-loaded Durability NPC-travel fixture shares this PostgreSQL harness.
#[allow(dead_code)] // Each standalone target uses only part of the shared fixture API.
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
