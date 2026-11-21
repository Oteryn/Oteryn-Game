//! Poem and telescope physical custody for the owned, selected Treasure Hunt route.
use super::l6_treasure_rewards_fixtures::{TILE};

use super::TestResult;

#[test]
fn l6_treasure_reward_9204_native_custody_and_replay() -> TestResult {
    run("l6-treasure-reward-9204")
}

#[test]
fn l6_treasure_reward_9616_native_custody_and_replay() -> TestResult {
    run("l6-treasure-reward-9616")
}

#[test]
fn l6_treasure_reward_9617_native_custody_and_replay() -> TestResult {
    run("l6-treasure-reward-9617")
}

#[test]
fn l6_treasure_reward_9618_native_custody_and_replay() -> TestResult {
    run("l6-treasure-reward-9618")
}

#[test]
fn l6_treasure_reward_9619_native_custody_and_replay() -> TestResult {
    run("l6-treasure-reward-9619")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Treasure reward custody requires PostgreSQL"
        );
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, TILE, false)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
