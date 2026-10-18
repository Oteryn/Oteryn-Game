#![allow(clippy::expect_used)]
// Standalone target for local focused runs. Native planning tests only despite the allocated
// filename; no PostgreSQL/writer result. The cases are included by `durability_postgres` so the
// protected CI lane runs the same tests.
pub use oteryn_game_server::{domain, durability};

#[path = "support/item_timed_postgres_cases.rs"]
mod item_timed_postgres_cases;
