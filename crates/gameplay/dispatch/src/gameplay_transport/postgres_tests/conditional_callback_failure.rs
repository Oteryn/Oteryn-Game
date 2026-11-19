//! Failed STEP callbacks use an authored safe landing, without earning quest progress.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn conditional_quest_failure_uses_authored_landing_and_replays_once() -> TestResult {
    scenario("conditional-failure-landing")
}

#[test]
fn conditional_quest_failure_refuses_missing_landing_without_quest_writes() -> TestResult {
    scenario("conditional-failure-blocked")
}

fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(
            name,
            Position {
                x: 32818,
                y: 31598,
                floor: -9,
            },
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
