//! The first carried Forbidden Fruit consumes one item and grants the achievement.
use super::TestResult;

#[test]
fn forbidden_fruit_grants_only_on_the_first_consumption() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-forbidden-fruit")
}
