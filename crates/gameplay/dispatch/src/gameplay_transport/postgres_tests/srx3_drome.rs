//! SRX-3: the accepted arena rules count completed zero-based waves.
use super::TestResult;

#[test]
fn srx3_drome_postgres_wave_zero_real_dispatch_and_native_combat() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| super::reaudit_arena_lifecycle_fixtures::physical().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "SRX-3 Drome production scenario panicked")??;
    eprintln!(
        "SRX3_DROME POSTGRES_CASE_COMPLETED wave_zero_native_combat duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn srx3_drome_postgres_reaching_wave_300_ends_without_spawning_it() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::reaudit_arena_lifecycle_fixtures::physical_at(299).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "SRX-3 Drome ceiling scenario panicked")??;
    eprintln!(
        "SRX3_DROME POSTGRES_CASE_COMPLETED reaching_wave_300 duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn srx3_drome_postgres_public_entry_refuses_unqualified_pool_and_room() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::reaudit_arena_lifecycle_fixtures::physical_unqualified()
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "unqualified Drome admission test panicked")??;
    eprintln!(
        "SRX3_DROME POSTGRES_CASE_COMPLETED public_unqualified_refusal duration={:?}",
        started.elapsed()
    );
    Ok(())
}
