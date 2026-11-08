//! A live paid quote retains confirmation ownership before reward fallbacks.
use super::TestResult;

#[test]
fn engnpc_paid_confirmation_precedes_reward_keyword_fallback() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .name("engnpc-paid-confirmation".into())
        .stack_size(256 << 20)
        .spawn(|| super::npc_paid_travel::confirmation_scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "paid confirmation thread failed")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED engnpc_paid_confirmation_precedes_reward_keyword_fallback duration={:?}",
        started.elapsed()
    );
    Ok(())
}
