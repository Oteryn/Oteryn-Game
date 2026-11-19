use super::TestResult;

#[test]
fn fortress_room_rejects_a_second_player_and_retires_exact_boss_on_exit_or_timeout() -> TestResult {
    super::rookie_guard_fortress_room_fixtures::run_case()
}
