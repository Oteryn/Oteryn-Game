// Dedicated PostgreSQL 17.6 qualification for INBOX-1a (migration 0076).
// Ordinary workspace runs report PRE-ROUTING/NONCANONICAL when the routed
// database is absent.

// Standalone target for local focused runs. The cases use SQL only, so the
// protected PostgreSQL lane includes the same file through its own wrapper.
#[path = "support/character_inbox_postgres_cases.rs"]
mod character_inbox_postgres_cases;
