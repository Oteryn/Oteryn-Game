//! Stateless Premium admission through literal STEP dispatch and the live account owner.
use super::{TestResult, conditional_callback_failure::scenario};

#[test]
fn stateless_premium_step_uses_live_admission_and_never_writes_quest_state() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    scenario("conditional-failure-premium-free")?;
    scenario("conditional-failure-premium-current")
}
