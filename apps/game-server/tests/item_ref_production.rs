#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17 qualification for MAP-ITEM-REF-1 Part B (the capability-4 corpse
// read and take over a bound corpse MINT) and for EQUIP-WIRE-1 (command 9's equip and unequip
// through the equipment owner). Ordinary workspace runs report
// PRE-ROUTING/NONCANONICAL when the routed database is absent; the protected lane runs the same
// cases through character_authority_postgres.
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

#[path = "support/item_transfer_postgres_cases.rs"]
mod item_transfer_postgres_cases;

#[path = "support/corpse_transfer_postgres_cases.rs"]
mod corpse_transfer_postgres_cases;

#[path = "support/item_ref_production_postgres_cases.rs"]
mod item_ref_production_postgres_cases;

#[path = "support/equipment_move_postgres_cases.rs"]
mod equipment_move_postgres_cases;

#[allow(dead_code, unused_imports)]
#[path = "support/content_spell_data_shim.rs"]
mod content;
