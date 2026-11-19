//! Koshei's source payment uses the existing fenced fee/input/quest participants.

use super::TestResult;

#[test]
fn qnpcfx_koshei_paid_pieces_atomic_replay_and_clock() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qnpcfx-koshei")
}

#[test]
fn qnpcfx_koshei_mixed_carried_bank_payment_atomic() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qnpcfx-koshei-bank")
}
