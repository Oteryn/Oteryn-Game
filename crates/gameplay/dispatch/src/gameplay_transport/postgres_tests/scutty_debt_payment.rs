use super::TestResult;
use super::scutty_debt_payment_fixtures::NAME;

#[test]
fn scutty_confirmation_charges_carried_gold_and_clears_debt_atomically() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards_fixtures::scenario(NAME)
}
