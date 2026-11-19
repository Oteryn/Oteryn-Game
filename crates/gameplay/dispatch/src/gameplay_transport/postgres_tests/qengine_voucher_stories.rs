//! Complete source item-grant stories against the stock admitted generation.

use super::TestResult;

#[test]
fn qengine_l9_afflicted_complete_native_item_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l9-afflicted-story")
}
#[test]
fn qengine_l9_soil_guardian_complete_native_voucher_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l9-soil-guardian-story")
}
#[test]
fn qengine_l9_crystal_warlord_complete_native_voucher_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l9-crystal-warlord-story")
}

#[test]
fn qengine_l9b_soil_guardian_native_supply_package_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l9-soil-guardian-package-story")
}
#[test]
fn qengine_l9b_crystal_warlord_native_supply_package_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l9-crystal-warlord-package-story")
}
