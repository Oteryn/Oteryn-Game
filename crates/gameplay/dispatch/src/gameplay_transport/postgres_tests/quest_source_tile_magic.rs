use super::TestResult;
use super::quest_source_tile_magic_fixtures::NAME;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn quest_source_tile_magic_and_monster_say_follow_committed_use() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("quest-source-tile-magic", || {
        super::quest_world_callbacks_fixtures::scenario(
            NAME,
            Position {
                x: 32000,
                y: 32000,
                floor: -7,
            },
            false,
        )
    })
}
