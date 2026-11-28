use super::TestResult;

#[test]
fn billy_exchanges_a_pan_for_a_small_health_potion_and_refuses_without_one() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-rookgaard-billy")
}

#[test]
fn lee_delle_exchanges_a_honey_flower_for_studded_legs() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-rookgaard-lee-delle")
}
