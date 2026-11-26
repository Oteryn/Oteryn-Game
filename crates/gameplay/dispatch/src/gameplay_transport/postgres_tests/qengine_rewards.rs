//! QENGINE-2: source rewards through the production NPC command dispatcher.
use super::qengine_rewards_fixtures::scenario;

use super::TestResult;



#[path = "qengine_rewards/m39_any_topic.rs"]
mod m39_any_topic;

#[test]
fn qengine_rewards_pirate_outfit() -> TestResult {
    scenario("qengine-rewards-pirate")
}
#[test]
fn qengine_rewards_knight_outfit() -> TestResult {
    scenario("qengine-rewards-knight")
}
#[test]
fn qengine_rewards_deepling_outfit() -> TestResult {
    scenario("qengine-rewards-deepling")
}
#[test]
fn qengine_rewards_hallowed_ground_exchange() -> TestResult {
    scenario("qengine-rewards-hallowed")
}
#[test]
fn qengine_rewards_eroth_starts_mission_and_issues_mirror() -> TestResult {
    scenario("qengine-rewards-eroth-start")
}

#[test]
fn qengine_rewards_eroth_legacy_save_uses_tagged_crystal() -> TestResult {
    scenario("qengine-rewards-eroth-legacy")
}

#[test]
fn qengine_rewards_one_eyed_joe_accepts_cursed_crystal_mission() -> TestResult {
    scenario("qengine-rewards-joe-start")
}

#[test]
fn qengine_rewards_outfox_exchange() -> TestResult {
    scenario("qengine-rewards-outfox")
}
#[test]
fn qengine_rewards_backpack_exchange() -> TestResult {
    scenario("qengine-rewards-backpack")
}

#[test]
fn qengine_l1_warmaster_crest() -> TestResult {
    scenario("qengine-rewards-l1-warmaster")
}
#[test]
fn qengine_l1_wayfarer_cape() -> TestResult {
    scenario("qengine-rewards-l1-wayfarer")
}
#[test]
fn qengine_l1_yalaharian_addon_sage() -> TestResult {
    scenario("qengine-rewards-l1-yalaharian")
}

#[test]
fn qengine_l1_warmaster_tribal() -> TestResult {
    scenario("qengine-rewards-l1-warmaster-tribal")
}
#[test]
fn qengine_l1_wayfarer_hat() -> TestResult {
    scenario("qengine-rewards-l1-wayfarer-hat")
}
#[test]
fn qengine_l1_yalaharian_base_palimuth() -> TestResult {
    scenario("qengine-rewards-l1-yalaharian-base-palimuth")
}
#[test]
fn qengine_l1_yalaharian_base_palimuth_accepts_villain() -> TestResult {
    scenario("qengine-rewards-l1-yalaharian-base-palimuth-villain")
}
#[test]
fn qengine_l1_yalaharian_base_yalahari() -> TestResult {
    scenario("qengine-rewards-l1-yalaharian-base-yalahari")
}
#[test]
fn qengine_l1_yalaharian_addon_villain() -> TestResult {
    scenario("qengine-rewards-l1-yalaharian-villain")
}

#[test]
fn qengine_l1_legacy_decaying_entitlement_remains_selectable() -> TestResult {
    scenario("qengine-rewards-l1-legacy-decaying")
}

#[path = "qengine_rewards/questl2_passages.rs"]
mod questl2_passages;
