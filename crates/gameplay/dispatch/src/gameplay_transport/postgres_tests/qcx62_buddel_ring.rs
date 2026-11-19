//! Crystal 201189f8 data-global/npc/buddel.lua:59-67,80-91.
use super::TestResult;
use super::qcx62_buddel_ring_fixtures::NAME;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn buddel_exchanges_exactly_one_carried_ring_for_the_named_island() -> TestResult {
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
