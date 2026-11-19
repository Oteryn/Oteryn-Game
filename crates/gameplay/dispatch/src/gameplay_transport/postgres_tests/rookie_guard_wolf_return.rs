use super::TestResult;

#[test]
fn wolf_escape_timer_projects_return_movement_and_allows_only_unfinished_attempts_to_retry() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::rookie_guard_wolf_return_fixtures::run_case()
}
