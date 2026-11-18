//! The source repair timer must publish alongside shared encounter admission.
use super::TestResult;

#[test]
fn lersatio_shared_encounter_source_repairs_on_deadline() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::timed_source_repair()
}
