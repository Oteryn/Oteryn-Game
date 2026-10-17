// Focused qualification keeps the original strict missing-PG refusal. Ordinary
// workspace runs execute these cases through durability_postgres instead.
use oteryn_game_server::{foundation, house_scope};

const HOUSE_RUNTIME_POSTGRES_REQUIRED: bool = true;

#[path = "support/house_runtime_postgres_cases.rs"]
mod house_runtime_postgres_cases;
