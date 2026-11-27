use super::TestResult;

#[test]
fn classic_prisoner_takes_seven_apples_and_mints_numbered_key_after_confirmations() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::qengine_rewards_fixtures::scenario("qengine-rewards-classic-prisoner")
}
