//! Generic binding fixture only; this is not source authoring for the packet quest.

use super::TestResult;

#[test]
fn authored_bindings_native_kill_threshold_completion_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-authored-kills")
}

#[test]
fn authored_bindings_queued_kill_burst_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-authored-kills-burst")
}
