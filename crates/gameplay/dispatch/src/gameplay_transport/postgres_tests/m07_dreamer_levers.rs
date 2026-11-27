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

#[test]
fn dread_lord_lever_burns_diamond_creates_ground_item_and_resets_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("dread-lord-lever", || {
        super::quest_world_callbacks_fixtures::scenario(
            "dreamer-dread-lord-scroll",
            super::m07_dreamer_levers_fixtures::DREAD_LORD_LEVER,
            false,
        )
    })
}

#[test]
fn dread_lord_lever_releases_reservation_on_terminal_retirement_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    const NAME: &str = "dreamer-dread-lord-scroll/retirement";
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(
            NAME,
            super::m07_dreamer_levers_fixtures::DREAD_LORD_LEVER,
            false,
        )
    })
}
