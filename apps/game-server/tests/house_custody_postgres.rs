// Dedicated PostgreSQL 17.6 qualification for HOUSE-CUSTODY-1 (migration
// 0025). Ordinary workspace runs report PRE-ROUTING/NONCANONICAL when the
// routed database is absent.

// Standalone target for local focused runs. The cases use SQL only, so the
// protected PostgreSQL lane includes the same file through its own wrapper.
#[path = "support/house_custody_postgres_cases.rs"]
mod house_custody_postgres_cases;
