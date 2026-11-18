//! Literal UID2244 on synthetic geometry: a real item drop followed by UID9234 STEP.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn m07_ring_real_additem_consumes_once_then_step_resets_shared_trees() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m07-ring-offering", || {
        super::quest_world_callbacks_fixtures::scenario(
            "m07-ring-offering",
            Position {
                x: 100,
                y: 100,
                floor: -11,
            },
            false,
        )
    })
}
