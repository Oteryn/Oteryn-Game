use super::{TestResult, qengine_rewards_fixtures};

#[test]
fn storkus_refuses_direct_dust_and_commits_collection_before_report() -> TestResult {
    qengine_rewards_fixtures::scenario("qengine-rewards-storkus-direct-dust")
}

#[test]
fn storkus_completes_dust_mission_without_henricus_referral() -> TestResult {
    qengine_rewards_fixtures::scenario("qengine-rewards-storkus-dust-without-referral")
}
