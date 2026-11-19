//! Crystal 201189f8 data-global/scripts/actions/tools/skinning.lua:259-268.
use super::TestResult;
use super::qespike_ashes_fixtures::NAME;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn ashes_click_and_hotkey_share_the_age_gate_and_preserve_knife_and_corpse() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(
            NAME,
            Position {
                x: 7,
                y: 0,
                floor: -7,
            },
            false,
        )
    })
}
