//! A live Albinius offering after the quest definition gains another transition.
//! Crystal 201189f8, data-global/npc/albinius.lua:172-192 and 194-214.
use super::TestResult;
use super::quest_definition_predecessors_fixtures::CASE;

#[test]
fn albinius_offerings_accept_existing_tome_progress_after_definition_change() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards_fixtures::scenario(CASE)
}
