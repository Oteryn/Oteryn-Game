use super::TestResult;

#[test]
fn plague_entrance_consumes_neither_pearl_until_both_are_present() -> TestResult {
    super::banshee_seal_offerings_fixtures::run_plague()
}

#[test]
fn sacrifice_blood_pour_signals_and_flame_consumes_only_the_blood_splash() -> TestResult {
    super::banshee_seal_offerings_fixtures::run_sacrifice()
}
