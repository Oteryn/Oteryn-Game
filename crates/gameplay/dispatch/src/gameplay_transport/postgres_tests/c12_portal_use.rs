//! Both directions of a quest teleport use the production world-object owners.
use super::{TestResult, quest_world_callbacks_fixtures::scenario};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn quest_portal_use_teleports_with_busy_durability_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    for (name, x, floor) in [
        ("dryad-garden-enter#busy-holder", 33264, -7),
        ("dryad-garden-leave#busy-holder", 33202, -11),
    ] {
        super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
            scenario(name, Position { x, y: 32012, floor }, false)
        })?;
        eprintln!("C12_PORTAL_POSTGRES_CASE_COMPLETED {name}");
    }
    Ok(())
}
