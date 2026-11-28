//! Full level-qualified NPC and carried-tool programmes on the stock generation.

use super::TestResult;

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
fn qengine_l11_makeshift_complete_apprenticeship_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l11-makeshift-story")
}
