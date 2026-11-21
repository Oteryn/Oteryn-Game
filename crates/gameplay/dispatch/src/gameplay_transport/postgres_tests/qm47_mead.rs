//! M47 drives the fenced world writer, preserving the command receipt on replay.
use super::TestResult;

use super::quest_world_callbacks_fixtures::scenario;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn m47_postgres_mead_roll_is_fenced_and_exactly_replayed() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m47-mead", || {
        scenario(
            "barbarian-test-mead",
            Position {
                x: 32201,
                y: 31154,
                floor: -7,
            },
            false,
        )
    });
    println!(
        "M47_POSTGRES_EXECUTED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    result
}
