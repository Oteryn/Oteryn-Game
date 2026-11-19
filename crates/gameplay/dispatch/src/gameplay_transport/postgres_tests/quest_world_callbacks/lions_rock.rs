use super::*;

#[test]
fn lions_rock_postgres_entry_refusal_has_one_message_and_no_state_change() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("lions-rock-entry-refusal", || {
        super::scenario(
            "lions-rock-inner-sanctum-entry",
            Position {
                x: 33128,
                y: 32308,
                floor: -8,
            },
            false,
        )
    })
}
