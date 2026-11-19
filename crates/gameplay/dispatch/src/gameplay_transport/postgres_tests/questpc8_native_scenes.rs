//! Source-literal scene edits through the normal PostgreSQL-backed USE route.
use super::questpc8_native_scenes_fixtures::{LEVER};

use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_mintwallin_lever_edits_all_walls_once_and_restores_with_floor() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-mintwallin-scene", || {
        quest_world_callbacks::scenario(
            "mintwallin-cyclops-scene",
            Position {
                x: LEVER.x,
                y: LEVER.y,
                floor: LEVER.floor,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_mintwallin_scene duration={:?}",
        started.elapsed()
    );
    Ok(())
}
