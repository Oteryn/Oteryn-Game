use super::TestResult;
use super::reaudit_arena_lifecycle_fixtures::{Physical, physical_context};

#[test]
fn n5_drome_postgres_claim_after_rotation_and_channel_restart() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            physical_context(0, true, false, Physical::RotationRestart)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Drome rotation restart thread panicked")??;
    eprintln!(
        "N5_POSTGRES_CASE_COMPLETED drome_claim_after_rotation_and_channel_restart duration={:?}",
        started.elapsed()
    );
    Ok(())
}
