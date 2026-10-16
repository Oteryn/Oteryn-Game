//! Native PostgreSQL-only root for the real private raid transition/store implementation.
#![allow(
    dead_code,
    reason = "Private raid data qualification; runtime activation remains gated"
)]
use crate::production_server::content::ProductionKey;
include!("core.rs");
#[path = "postgres_tests.rs"]
mod postgres_tests;
#[path = "test_support.rs"]
mod test_support;
