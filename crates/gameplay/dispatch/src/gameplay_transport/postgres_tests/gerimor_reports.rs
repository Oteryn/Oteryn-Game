use super::{TestResult, gerimor_reports_fixtures};

#[test]
fn gerimor_reports_preserve_prior_assignment_pin_and_settle_finale_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    gerimor_reports_fixtures::gerimor_reports_preserve_prior_assignment_pin_and_settle_finale_once_scenario()
}
