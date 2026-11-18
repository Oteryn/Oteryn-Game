//! Generic binding execution fixtures; these are not source authoring for the named quest.

use super::TestResult;

#[test]
fn authored_bindings_npc_delivery_and_carried_use_dispatch_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-authored-bindings")
}
