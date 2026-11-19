//! Crystal 201189f8 data-global/npc/iskan.lua:53-127 through fenced NPC dispatch.
use super::TestResult;
use super::qcx62_iskan_fixtures::{LANDING, NAME};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn iskan_greets_earned_warlords_and_requires_the_musher_report_for_passage() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(
            NAME,
            Position {
                x: 32322,
                ..LANDING
            },
            false,
        )
    })
}
