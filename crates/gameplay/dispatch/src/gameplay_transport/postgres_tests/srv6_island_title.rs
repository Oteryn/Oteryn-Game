//! SRV6: the level-999 source STEP commits the visit that command 113 consumes.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn srv6_robinson_crusoe_real_step_and_title_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("srv6-robinson-title", || {
        super::quest_world_callbacks_fixtures::scenario(
            "srv6-schrodinger-island",
            Position {
                x: 32717,
                y: 32345,
                floor: -7,
            },
            false,
        )
    })
}
