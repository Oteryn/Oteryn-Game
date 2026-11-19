//! A live paid quote owns its confirmation even when the NPC also has quest rewards.
use super::TestResult;

#[test]
fn engnpc_live_paid_quote_owns_yes_with_reward_callback() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::npc_paid_travel_fixtures::scenario_with_reward_callback()
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "paid confirmation PG thread failed")??;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "ENGNPC_PG_EXECUTED paid_confirmation duration={:?}",
        started.elapsed()
    );
    Ok(())
}
