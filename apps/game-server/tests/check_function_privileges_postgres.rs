// Dedicated PostgreSQL 17.6 qualification for PRIV-GUARD-1. Ordinary workspace
// runs report PRE-ROUTING/NONCANONICAL when the routed database is absent.

// Standalone target for local focused runs. The cases use SQL only, so the
// protected PostgreSQL lane includes the same file through its own wrapper.
#[path = "support/check_function_privileges_postgres_cases.rs"]
mod check_function_privileges_postgres_cases;
