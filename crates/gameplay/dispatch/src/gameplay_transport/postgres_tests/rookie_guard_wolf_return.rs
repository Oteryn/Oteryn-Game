use super::TestResult;

#[test]
fn wolf_escape_timer_projects_return_movement_and_allows_only_unfinished_attempts_to_retry() -> TestResult {
    super::rookie_guard_wolf_return_fixtures::run_case()
}
