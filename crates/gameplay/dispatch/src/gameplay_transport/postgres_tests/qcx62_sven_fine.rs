//! Crystal 201189f8 data-global/npc/sven.lua:117-127 through TALK and the fee owner.
use super::TestResult;
use super::qcx62_sven_fine_fixtures::NAME;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn sven_charges_each_reported_husky_before_clearing_the_debt() -> TestResult {
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
