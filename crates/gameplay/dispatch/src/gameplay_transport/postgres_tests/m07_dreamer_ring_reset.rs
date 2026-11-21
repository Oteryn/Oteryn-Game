//! Literal UID9234 on synthetic source geometry; real STEP restores only missing trees.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn m07_death_ring_real_step_restores_only_missing_trees_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m07-ring-reset", || {
        super::quest_world_callbacks_fixtures::scenario(
            "dreamers-challenge-death-ring-reset",
            Position {
                x: 100,
                y: 100,
                floor: -11,
            },
            false,
        )
    })
}
