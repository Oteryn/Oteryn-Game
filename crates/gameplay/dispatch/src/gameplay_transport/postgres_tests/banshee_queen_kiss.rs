use super::TestResult;

#[test]
fn queen_kiss_refuses_pz_lock_then_commits_arrival_and_three_seal_flags() -> TestResult {
    super::banshee_queen_kiss_fixtures::run_case()
}
