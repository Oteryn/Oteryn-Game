//! M09: two independent source-room caches, never one invented multi-item chest.

use super::TestResult;

#[test]
fn m09_treasure_telescope_once_and_replay() -> TestResult {
    run("m09-treasure-telescope")
}

#[test]
fn m09_treasure_three_ingots_once_and_replay() -> TestResult {
    run("m09-treasure-ingots")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with(name))
        .ok_or("treasure callback")?;
    let trigger = callback
        .selector
        .position
        .ok_or("treasure chest position")?;
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, trigger, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
