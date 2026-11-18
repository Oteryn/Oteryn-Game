//! M10 real NPC wire commands compose fee/input custody, quest progress and visual receipts.

use super::TestResult;

#[test]
fn qnpcfx_m10_oldrak_repeatable_payment_items_and_npc_magic() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qnpcfx-m10-oldrak")
}
#[test]
fn qnpcfx_m10_inquisition_five_real_interviews_progress_and_player_magic() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qnpcfx-m10-inquisition")
}
