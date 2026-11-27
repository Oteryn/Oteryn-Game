//! The Bear Room source lever opens and restores the retained stone tile.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn bear_room_source_switch_opens_and_restores_the_passage() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("bear-room-switch", || {
        super::quest_world_callbacks_fixtures::scenario(
            "bear-room-stone-switch",
            Position { x: 32148, y: 32105, floor: -11 },
            false,
        )
    })
}
