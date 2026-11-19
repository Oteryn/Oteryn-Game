//! Crystal 201189f8 data-global/npc/buddel.lua:54-76,80-104 through paid TALK.
use super::TestResult;
use super::qcx62_buddel_fixtures::NAME;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn buddel_rechecks_helheim_progress_before_paying_and_places_the_diversion() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(
            NAME,
            Position {
                x: 32256,
                y: 31193,
                floor: -7,
            },
            false,
        )
    })
}
