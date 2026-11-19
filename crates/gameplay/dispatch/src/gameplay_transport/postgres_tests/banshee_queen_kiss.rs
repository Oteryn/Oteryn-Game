use super::TestResult;

#[test]
fn queen_kiss_refuses_pz_lock_then_commits_arrival_and_three_seal_flags() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::banshee_queen_kiss_fixtures::run_case()
}
