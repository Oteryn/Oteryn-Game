use super::TestResult;

#[test]
fn paulie_credits_the_quest_bonus_only_with_the_first_successful_deposit() -> TestResult {
    super::rookie_guard_bank_bonus_fixtures::run_case()
}
