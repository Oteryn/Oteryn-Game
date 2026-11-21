//! Whole fixed Feaster of Souls expedition and actual native powerful learning.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_feaster_native_sampling_prefix() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-feaster-prefix", || {
        super::quest_world_callbacks_fixtures::scenario(
            "qengine-rewards-feaster-prefix",
            Position {
                x: 32627,
                y: 32076,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_FEASTER_PREFIX POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
