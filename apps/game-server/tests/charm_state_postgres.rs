#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17.6 qualification for CHARM-3 (migration 0020). Ordinary workspace runs
// report PRE-ROUTING/NONCANONICAL when the routed database is absent.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;

pub use production_server::admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
#[path = "support/domain_shim.rs"]
pub mod domain;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
pub use oteryn_gameplay_native_admission_source as native_admission_source;

#[path = "../src/gameplay_transport/charm.rs"]
mod charm_transport;
#[allow(dead_code, unused_imports)]
#[path = "../src/combat.rs"]
pub mod combat;

// Standalone target for local focused runs. The cases need only the path-loaded crate root
// above, so a protected PostgreSQL wrapper can include the same file.
#[path = "support/charm_state_postgres_cases.rs"]
mod charm_state_postgres_cases;

#[allow(dead_code, unused_imports)]
#[path = "support/content_spell_data_shim.rs"]
mod content;

// The path-loaded Durability NPC-travel fixture shares this PostgreSQL harness.
#[allow(dead_code)] // Each standalone target uses only part of the shared fixture API.
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
