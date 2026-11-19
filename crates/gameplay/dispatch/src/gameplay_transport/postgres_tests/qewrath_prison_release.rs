//! Crystal 201189f8 data-global/scripts/quests/wrath_of_the_emperor/movements_prison.lua:3-27.

use super::TestResult;
use super::qewrath_prison_release_fixtures::SOURCE;

#[test]
fn prison_release_restores_outfit_and_returns_the_crate_only_during_mission_one() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("qewrath-prison-release", || {
        for name in ["qewrath-prison-with-crate", "qewrath-prison-later"] {
            super::quest_world_callbacks_fixtures::scenario(name, SOURCE, false)?;
        }
        Ok(())
    })
}
