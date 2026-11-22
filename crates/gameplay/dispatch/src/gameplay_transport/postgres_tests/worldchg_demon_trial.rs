use super::TestResult;

#[test]
fn demon_wars_priestess_report_requires_ordered_native_deaths() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l12-demon-wars-story")
}
