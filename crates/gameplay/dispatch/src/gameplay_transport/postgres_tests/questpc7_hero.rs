//! Selected Rathleton investigation through native commands and death settlement.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_hero_full_native_investigation() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-hero", || {
        super::quest_world_callbacks_fixtures::scenario(
            "questpc7-hero",
            Position {
                x: 33564,
                y: 31951,
                floor: -14,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_HERO POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
