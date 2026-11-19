use super::TestResult;

#[test]
fn fleshy_bone_use_retains_the_item_and_recovers_the_guard_after_two_minutes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::rookie_guard_distraction_fixtures::run_case()
}
