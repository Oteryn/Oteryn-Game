//! Literal source rite with five genuine Fresh Characters, owned builds and altar custody.
use super::desert_party_rite_fixtures::{scenario};

use super::TestResult;

#[test]
fn desert_dungeon_five_station_rite_uses_real_fenced_players() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Desert party scenario panicked")??;
    println!(
        "DESERT_PARTY_RITE POSTGRES_CASE_COMPLETED duration_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
