//! Full level-qualified NPC and carried-tool programmes on the stock generation.

use super::TestResult;

#[test]
fn rotem_valos_does_not_offer_unimplemented_cartography() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::qengine_rewards_fixtures::scenario("qengine-rewards-cartography-held")
}

#[test]
fn qengine_l11_dream_warrior_complete_level_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l11-dream-warrior-story")
}
#[test]
fn qengine_l11_dream_warrior_undal_rewards_addons_in_reverse_order() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l11-dream-warrior-winter-story")
}
#[test]
fn qengine_l11_issavi_complete_level_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l11-issavi-story")
}
#[test]
fn qengine_l11_makeshift_uses_boss_tokens_and_earns_achievement_with_both_addons() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l11-makeshift-story")
}

#[test]
fn newhaven_gustavo_published_save_earns_ring_and_xp_once() -> TestResult {
    super::qengine_rewards_fixtures::scenario(super::newhaven_gustavo_fixtures::PUBLISHED_SAVE)
}
