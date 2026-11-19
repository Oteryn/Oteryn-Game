//! Shared clock engine only; no requesting lane's quest story is qualified here.

use super::TestResult;

#[test]
fn world_callback_clock_mint_cooldown_and_replay_dispatch_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-world-clock")
}

#[test]
fn world_callback_clock_full_bag_rolls_back_timestamp_dispatch_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-world-clock-full")
}
