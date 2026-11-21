//! Real published World regular minion onThink -> canonical named icon + KV.
use super::srx3_rotten_area_think_fixtures::{RottenWorldCase, run_rotten_world_case};

use super::TestResult;

#[test]
fn srx3_rotten_area_native_reserved_think_publishes_canonical_icon_and_kv_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    run_rotten_world_case(RottenWorldCase::InitialCharge)
}
