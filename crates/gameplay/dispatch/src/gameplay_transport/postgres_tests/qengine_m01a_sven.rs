//! Premium-gated chosen Barbarian Test starter through production TALK dispatch.

use super::TestResult;

#[test]
fn m01a_sven_premium_production_talk_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m01a-sven")
}
