//! Whole selected personal Grimvale programme; subject state is earned natively.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_grimvale_whole_native_investigation_and_rewards() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-grimvale-story", || {
        super::quest_world_callbacks_fixtures::scenario(
            "qengine-rewards-grimvale-story",
            Position {
                x: 33328,
                y: 31690,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_GRIMVALE_STORY POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
