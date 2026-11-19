//! Stag's source reward has neutral armour and an ordinary ammo-slot move event.
use super::l6_stag_item_fixtures::{TILE};

use super::TestResult;

#[test]
fn l6_stag_source_sword_has_native_custody_and_exact_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6-stag-item-profile");
    super::kill_reward_e2e_fixtures::scenario_thread::run("l6-stag-item-profile", ||
        super::quest_world_callbacks_fixtures::scenario("l6-stag-item-profile", TILE, false))?;
    eprintln!("POSTGRES_CASE_COMPLETED l6-stag-item-profile duration={:?}", started.elapsed());
    Ok(())
}
