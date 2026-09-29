// Dedicated PostgreSQL 17.6 qualification for STANCE-0 (migration 0017).
// Ordinary workspace runs report PRE-ROUTING/NONCANONICAL when the routed
// database is absent.

// Standalone target for local focused runs. The cases use SQL only, so the
// protected PostgreSQL lane can include the same file through its own wrapper.
#[path = "support/character_stance_postgres_cases.rs"]
mod character_stance_postgres_cases;
