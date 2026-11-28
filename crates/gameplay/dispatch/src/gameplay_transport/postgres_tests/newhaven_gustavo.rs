use super::TestResult;

#[test]
fn gustavo_assigns_credits_twenty_muglex_and_rewards_once() -> TestResult {
    super::qengine_rewards_fixtures::scenario(super::newhaven_gustavo_fixtures::NAME)
}
