//! Native Hive USE proves safe egress without a live scent or open stage.
use super::worldchg_hive_egress_fixtures::{scenario_with_bindings};

use super::TestResult;

#[test]
fn worldchg_hive_native_gate_closed_stage_and_missing_scent_allow_egress_only() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("worldchg-hive-egress", scenario)
}
fn scenario() -> TestResult {
    scenario_with_bindings(false)
}
