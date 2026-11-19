//! Steal From Thieves (TibiaWiki Steal_From_Thieves_Quest): Ahmet's mission, the
//! Darama oasis bag hand-in and the global reward of 100 brown mushrooms and 100 gold.

use super::TestResult;

#[test]
fn qd_steal_from_thieves_mission_bag_return_and_global_reward() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qd-steal-from-thieves")
}
