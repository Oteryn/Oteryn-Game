//! Crystal 201189f8 data-global/npc/sven.lua:70-150 through composed NPC TALK.
use super::TestResult;
use super::qcx62_sven_fixtures::NAME;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn sven_source_confirmation_and_reports_take_precedence_over_the_retained_starter() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(
            NAME,
            Position {
                x: 32243,
                y: 31056,
                floor: -7,
            },
            false,
        )
    })
}
