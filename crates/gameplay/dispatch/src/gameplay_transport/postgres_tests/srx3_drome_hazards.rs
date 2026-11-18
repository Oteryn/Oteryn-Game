//! Real command436 persists the full current-owner projection before immutable replay.

use super::*;
#[test]
fn srx3_drome_postgres_large_hazard_receipt_and_exact_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::reaudit_arena_lifecycle_fixtures::physical_hazards().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "large Drome receipt test panicked")??;
    eprintln!(
        "SRX3_DROME POSTGRES_CASE_COMPLETED large_hazard_receipt_exact_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}
