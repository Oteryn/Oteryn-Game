//! Production Item USE selects the same shared native owner and scope barrier.
use super::super::TestResult;
use super::super::soulpit_shared_entry_fixtures::native::{scenario};

#[test]
fn soulpit_native_item_use_entry_return_and_animus_query_postgres() -> TestResult {
    scenario(false, 32)
}

#[test]
fn soulpit_production_owner_spawns_native_wave_and_timeout_returns_without_mastery_postgres()
-> TestResult {
    scenario(true, 32)
}

#[test]
fn soulpit_native_entry_refuses_insufficient_wave_capacity_without_burning_core_postgres()
-> TestResult {
    scenario(false, 11)
}
