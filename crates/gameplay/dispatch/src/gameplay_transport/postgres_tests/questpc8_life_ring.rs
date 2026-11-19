//! Chosen Life Ring exploration, reusable pick, native descent and exact cache.

use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_life_ring_pick_descent_exact_rewards_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-life-ring", || {
        quest_world_callbacks::scenario(
            "authored-life-ring-entry",
            Position {
                x: 32437,
                y: 32238,
                floor: -10,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_life_ring_pick_descent_exact_rewards_and_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}
