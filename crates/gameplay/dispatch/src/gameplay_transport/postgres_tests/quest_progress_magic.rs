use super::TestResult;
use super::quest_progress_magic_fixtures::NAME;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn quest_step_in_hints_follow_committed_progress_and_reset() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("quest-progress-magic", || {
        super::quest_world_callbacks_fixtures::scenario(
            NAME,
            Position {
                x: 32991,
                y: 31496,
                floor: -1,
            },
            false,
        )
    })
}
