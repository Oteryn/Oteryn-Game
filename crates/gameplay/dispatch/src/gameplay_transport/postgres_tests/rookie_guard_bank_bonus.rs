use super::TestResult;

#[test]
fn paulie_credits_the_quest_bonus_only_with_the_first_successful_deposit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::rookie_guard_bank_bonus_fixtures::run_case()
}
