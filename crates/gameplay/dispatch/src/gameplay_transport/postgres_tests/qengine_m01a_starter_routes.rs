//! Selected M01a briefings through the production registry and NPC transport.

use super::TestResult;

#[test]
fn m01a_rafzan_baxter_emilie_production_talk_routes_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m01a-starter-routes")
}
