//! Chosen programmes use real movement, captured native deaths and literal USE.

use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_braindeath_exploration_native_death_coffin_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-braindeath", || {
        quest_world_callbacks::scenario(
            "authored-braindeath",
            Position {
                x: 32545,
                y: 31522,
                floor: -11,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_braindeath_exploration_native_death_coffin_and_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn questpc8_tinder_box_summit_and_once_reward() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-tinder", || {
        quest_world_callbacks::scenario(
            "authored-tinder-box",
            Position {
                x: 32084,
                y: 31025,
                floor: -1,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_tinder_box_summit_and_once_reward duration={:?}",
        started.elapsed()
    );
    Ok(())
}
