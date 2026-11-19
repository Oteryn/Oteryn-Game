//! Exact donor Ahau entry and committed native STEP return, using the shared owner.
use super::m25_ahau_exit_fixtures::{scenario_case};

use super::TestResult;

#[test]
fn m25_ahau_native_step_exit_returns_exact_current_instance_participant_postgres() -> TestResult {
    scenario(false)
}
#[test]
fn m25_ahau_player_push_exit_returns_only_pushed_participant_postgres() -> TestResult {
    scenario(true)
}
fn scenario(push: bool) -> TestResult {
    scenario_case(push, false)
}
