//! Crystal 201189f8 data-global/npc/emiliana.lua:72-84,209-242.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn emiliana_briefs_in_crimson_court_only_from_the_designated_player_area() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("qewrath-emiliana", || {
        for (name, x, floor) in [
            ("qewrath-emiliana-inside", 32410, -12),
            ("qewrath-emiliana-outside", 32409, -12),
            ("qewrath-emiliana-surface", 32410, -7),
        ] {
            super::quest_world_callbacks_fixtures::scenario(
                name,
                Position { x, y: 32683, floor },
                false,
            )?;
        }
        Ok(())
    })
}
