//! Source-gated dialogue on a character whose saved quest predates the new route.

use super::TestResult;
use super::qewrath_npc_handoffs_fixtures::scenario;

#[test]
fn zalamon_opens_phantom_army_from_the_saved_mission_four_definition() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario("qewrath-npc-handoff-zalamon")
}

#[test]
fn sleeping_dragon_commits_charm_custody_and_mission_ten_together() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario("qewrath-npc-handoff-dragon")
}
