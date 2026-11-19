//! Crystal 201189f8, data-global/npc/albinius.lua:172-192.
use super::TestResult;
use super::albinius_ice_portal_fixtures::CASE;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn albinius_ice_offering_requires_formorgar_and_charges_fifty_fish_on_existing_tome_pin()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(CASE, || {
        super::quest_world_callbacks_fixtures::scenario(
            CASE,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}
