//! L7 source-literal lever/rock truth table through admitted USE dispatch.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn quest_l7_giant_smithhammer_four_source_states() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("l7-smithhammer", || {
        super::quest_world_callbacks_fixtures::scenario(
            "giant-smithhammer-asymmetric",
            Position {
                x: 32779,
                y: 32230,
                floor: -8,
            },
            false,
        )
    })
}
