use super::TestResult;

#[test]
fn plague_entrance_consumes_neither_pearl_until_both_are_present() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::banshee_seal_offerings_fixtures::run_plague()
}

#[test]
fn sacrifice_blood_pour_signals_and_flame_consumes_only_the_blood_splash() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::banshee_seal_offerings_fixtures::run_sacrifice()
}
