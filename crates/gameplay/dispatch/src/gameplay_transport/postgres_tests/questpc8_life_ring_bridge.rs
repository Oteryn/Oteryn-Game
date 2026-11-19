//! Six native bridge cells and top-creature evacuation through the real USE owner.

use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_life_ring_bridge_moves_occupants_with_all_six_cells() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-life-ring-bridge", || {
        quest_world_callbacks::scenario(
            "authored-life-ring-bridge",
            Position {
                x: 32414,
                y: 32231,
                floor: -10,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_life_ring_bridge_moves_occupants_with_all_six_cells duration={:?}",
        started.elapsed()
    );
    Ok(())
}
