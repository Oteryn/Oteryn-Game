use super::TestResult;

#[test]
fn spider_queen_wrap_waits_for_grease_expiry_and_sends_the_player_to_an_escapable_nest() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::rookie_guard_web_grease_fixtures::run_case()
}
