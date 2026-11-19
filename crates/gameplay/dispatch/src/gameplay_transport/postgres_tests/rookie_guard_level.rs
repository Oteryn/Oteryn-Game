use super::TestResult;

#[test]
fn vascalir_refuses_late_entry_and_records_level_eight_continuation() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::rookie_guard_level_fixtures::run_case()
}
