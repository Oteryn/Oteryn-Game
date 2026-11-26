use super::{TestResult, qengine_rewards_fixtures};

#[test]
fn storkus_direct_dust_hand_in_commits_debit_and_both_tracks_once() -> TestResult {
    qengine_rewards_fixtures::scenario("qengine-rewards-storkus-direct-dust")
}
