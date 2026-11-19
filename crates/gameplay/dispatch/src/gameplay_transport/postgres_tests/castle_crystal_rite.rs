use super::TestResult;
use super::castle_crystal_rite_fixtures::{NAME, STATIONS};

#[test]
fn castle_crystal_step_commits_four_station_rite_only_when_every_member_qualifies() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, STATIONS[0], false)
    })
}
