//! Source Dermot key acquisition: atomic payment, exact key, replay and repurchase.

use super::TestResult;

#[test]
fn questpc6_fibula_purchase_carried_price_exact_key_replay_and_repurchase() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-fibula-key-coins")
}

#[test]
fn questpc6_fibula_purchase_bank_price_exact_key_replay_and_repurchase() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-fibula-key-bank")
}

#[test]
fn questpc6_fibula_purchase_full_backpack_rolls_back_bank_fee_and_key() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-fibula-key-full")
}
