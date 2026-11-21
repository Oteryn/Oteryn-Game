//! Whole fixed Heart of Destruction expedition and actual native powerful learning.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_heart_whole_native_breaches_and_rewards() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-heart-story", || {
        super::quest_world_callbacks_fixtures::scenario(
            "qengine-rewards-heart-story",
            Position {
                x: 33026,
                y: 31367,
                floor: -8,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_HEART_STORY POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
