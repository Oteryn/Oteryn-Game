//! Complete chosen hoist knowledge programme and native position-bound transport.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_hoist_full_knowledge_control_completion_and_native_binding() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-hoist-story", || {
        super::quest_world_callbacks_fixtures::scenario(
            "questpc7-hoist-story",
            Position {
                x: 32154,
                y: 31118,
                floor: -9,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_HOIST_STORY POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
