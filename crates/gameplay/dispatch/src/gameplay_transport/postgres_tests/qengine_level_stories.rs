//! Full level-qualified NPC and carried-tool programmes on the stock generation.

use super::TestResult;

#[test]
fn qengine_l11_dream_warrior_complete_level_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l11-dream-warrior-story")
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
fn newhaven_gustavo_twenty_muglex_deaths_earn_ring_and_xp_once() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l11-newhaven-defence")
}
