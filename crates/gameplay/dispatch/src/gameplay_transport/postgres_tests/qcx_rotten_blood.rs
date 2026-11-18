//! Global Rotten Charge cadence through the native World think and durable quest clock.
use super::TestResult;

#[test]
fn passive_rotten_charge_advances_after_twenty_seconds() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::srx3_rotten_area_think::run_area_think_case(true)
}

#[test]
fn bakragore_admission_opens_a_twenty_five_minute_fight() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::srx3_rotten_advanced::bakragore_deadline_case()
}
