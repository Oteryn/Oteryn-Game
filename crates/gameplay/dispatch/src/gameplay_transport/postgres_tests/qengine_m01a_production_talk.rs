//! M01a production wiring: the fixed Torch mark starts through Smaralda's live NPC actor.

use super::TestResult;

#[test]
fn m01a_pirates_death_production_talk_dispatch_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m01a-production-pirate-talk")
}
