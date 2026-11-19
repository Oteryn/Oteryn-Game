use super::TestResult;

#[test]
fn fortress_room_rejects_a_second_player_and_retires_exact_boss_on_exit_or_timeout() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::rookie_guard_fortress_room_fixtures::run_case()
}
