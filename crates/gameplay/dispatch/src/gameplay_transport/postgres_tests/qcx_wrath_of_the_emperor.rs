//! Zalamon's entry gate and marked crate through the native NPC wire.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn children_of_the_revolution_completion_unlocks_zalamon() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("qcx-wrath-entry", || {
        super::quest_world_callbacks_fixtures::scenario(
            "authored-wrath-opening",
            Position {
                x: 33354,
                y: 31410,
                floor: -8,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_FINISHED qcx_wrath_entry duration={:?} result={result:?}",
        started.elapsed()
    );
    result
}
