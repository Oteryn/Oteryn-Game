//! M07: literal native lever USE preserves conditional source walls and independent timers.
use super::m07_dreamer_levers_fixtures::{CONTROL};

use super::TestResult;

use super::m07_dreamer_levers_fixtures::NAME;

#[test]
fn m07_dreamer_native_lever_conditional_walls_and_distinct_deadlines_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {NAME}");
    let result = (|| -> TestResult {
        std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
            super::quest_world_callbacks_fixtures::scenario(NAME,CONTROL,false).map_err(|e| e.to_string())
        })?.join().map_err(|_| "M07 Dreamer scenario thread panicked")??;
        Ok(())
    })();
    eprintln!("POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",started.elapsed(),result.is_ok());
    result
}
