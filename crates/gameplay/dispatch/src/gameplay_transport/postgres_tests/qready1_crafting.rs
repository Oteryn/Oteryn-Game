//! M20 World crafting against the unchanged native wooden grinder room.
use super::qready1_crafting_fixtures::{AT, NAME};

use super::TestResult;

#[test]
fn qready1_native_grinder_spends_each_splinter_and_mints_each_dust_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks_fixtures::scenario(NAME, AT, false).map_err(|e| e.to_string())
        })?
        .join()
        .unwrap_or_else(|_| Err("M20 grinder PG worker panicked".into()));
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
