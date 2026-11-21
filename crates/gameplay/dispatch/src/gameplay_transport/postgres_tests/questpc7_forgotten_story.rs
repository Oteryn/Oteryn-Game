//! Whole fixed Forgotten Knowledge expedition and actual native powerful learning.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_forgotten_whole_native_paths_and_learning() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-forgotten-story", || {
        super::quest_world_callbacks_fixtures::scenario(
            "qengine-rewards-forgotten-story",
            Position {
                x: 32327,
                y: 32093,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_FORGOTTEN_STORY POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
