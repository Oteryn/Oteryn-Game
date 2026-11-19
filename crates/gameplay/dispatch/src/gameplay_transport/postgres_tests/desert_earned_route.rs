//! Compose the source level gate, party rite and physical earned rewards.
use super::TestResult;

#[test]
fn desert_dungeon_native_gate_rite_earned_rewards_and_exit() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| super::desert_party_rite_fixtures::scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Desert earned route panicked")??;
    println!(
        "DESERT_EARNED_ROUTE POSTGRES_CASE_COMPLETED duration_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
