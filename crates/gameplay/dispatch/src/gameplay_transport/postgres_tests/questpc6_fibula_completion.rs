//! Continuous native portal, source key/cache collection, native deaths and completion.

use super::TestResult;

#[test]
fn questpc6_fibula_complete_native_path_level_claims_replay_and_paid_repurchase() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-fibula-completion")
}
#[test]
fn questpc6_fibula_completion_refuses_level_fifty_eight_kills_with_missing_claims() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-fibula-completion-missing-claims")
}
