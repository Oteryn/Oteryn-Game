//! Reuse questpc8's chosen smith cache through the existing native owners.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_smith_native_guardian_exact_cache_completion() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-smith", || {
        super::quest_world_callbacks_fixtures::scenario(
            "qengine-rewards-smith-story",
            Position {
                x: 32776,
                y: 32254,
                floor: -8,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_SMITH POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
