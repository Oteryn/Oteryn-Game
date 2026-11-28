//! Exercise the composed Drome wave tick with an explicit test-only creature table.
use super::{
    TestResult,
    reaudit_arena_lifecycle_fixtures::{Physical, physical_context},
};

const WAVE: u32 = 10;
#[test]
fn w17_drome_postgres_draws_native_keys_and_scales_spawned_health() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            physical_context(WAVE, true, false, Physical::NativeWave)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "W17 Drome wave scenario panicked")??;
    eprintln!(
        "W17_DROME_POSTGRES_CASE_COMPLETED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
