//! Isolated native XP/objective owner proof. Prior route progress is a fixture
//! precondition; this does not qualify Elvenbane traversal, combat or rewards.
use super::TestResult;

#[test]
fn native_objective_and_xp_share_one_postgres_transaction() -> TestResult {
    super::annihilator_route_fixtures::native_objective_atomicity()
}

#[test]
fn stale_native_objective_capture_is_terminal_on_xp_replay() -> TestResult {
    super::annihilator_route_fixtures::native_objective_stale_capture()
}
