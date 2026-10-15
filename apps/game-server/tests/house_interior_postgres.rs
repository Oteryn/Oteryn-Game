// HOUSE-RUNTIME-1a local preparation harness. These production modules are
// intentionally unregistered until the CP slot; no production writer is installed.
pub use oteryn_game_server::durability::{house_scope_handoff, item_mint_audit};
pub use oteryn_game_server::{domain, foundation};

#[path = "../src/durability/house_interior.rs"]
mod house_interior;
#[path = "../src/durability/house_interior_audit.rs"]
mod house_interior_audit;
#[path = "support/house_interior_postgres_cases.rs"]
mod house_interior_postgres_cases;
#[path = "support/house_test_fixture.rs"]
mod house_test_fixture;
