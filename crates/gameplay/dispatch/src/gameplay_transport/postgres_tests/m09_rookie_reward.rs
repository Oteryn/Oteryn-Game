//! M09: Vascalir's actual two-item mission acceptance on the production wire.

use super::TestResult;

#[test]
fn m09_rookie_guard_two_rewards_once_and_replay() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m09-rookie")
}
