use super::{TestResult, qengine_rewards_fixtures};

#[test]
fn storkus_refuses_direct_dust_and_commits_collection_before_report() -> TestResult {
    qengine_rewards_fixtures::scenario("qengine-rewards-storkus-direct-dust")
}
