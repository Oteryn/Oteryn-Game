//! Native cooking ingredients from existing Bat/Chicken definitions and ordinary deaths.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_hot_native_bat_chicken_corpse_inputs() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-hot-corpses", || {
        super::quest_world_callbacks_fixtures::scenario(
            "qengine-rewards-hot-corpses",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_HOT_CORPSES POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
