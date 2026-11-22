//! Full level-qualified NPC and carried-tool programmes on the stock generation.

use super::TestResult;

#[test]
fn rotem_valos_issues_one_adventurers_map() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::qengine_rewards_fixtures::scenario("qengine-rewards-cartography-assignment")
}

#[test]
fn qengine_l11_dream_warrior_complete_level_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l11-dream-warrior-story")
}
#[test]
fn qengine_l11_issavi_complete_level_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l11-issavi-story")
}
#[test]
fn qengine_l11_makeshift_complete_apprenticeship_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l11-makeshift-story")
}
