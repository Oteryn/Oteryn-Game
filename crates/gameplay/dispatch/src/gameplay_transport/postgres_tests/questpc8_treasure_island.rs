//! Chosen exploration-only journey drives both paid ship routes and the native sand/rope cells.

use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_treasure_island_paid_ship_map_shovel_rope_explore_and_return() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-treasure-island", || {
        quest_world_callbacks::scenario(
            "authored-treasure-island",
            Position {
                x: 32131,
                y: 32913,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_treasure_island_paid_ship_map_shovel_rope_explore_and_return duration={:?}",
        started.elapsed()
    );
    Ok(())
}
