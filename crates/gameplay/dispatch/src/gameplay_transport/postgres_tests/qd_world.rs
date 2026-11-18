//! Quest lane qd: travel, door, lever and chest routes over the shared world scenario.
//! Each case names its TibiaWiki source; the route is driven through NPC talk, the
//! placed quest door, a real step and USE on the production placement.
use super::qd_world_fixtures::{SAMS, start};

use super::TestResult;


#[test]
fn qd_sams_old_backpack_kroox_door_and_dwarven_armor_chest() -> TestResult {
    run(SAMS)
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, start(name)?, false)
    })
}
