use super::TestResult;
use super::lersatio_mirrors_fixtures::{MIRROR, NAME};

#[test]
fn lersatio_mirror_repairs_then_second_hit_moves_two_players_and_spawns_encounter() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, MIRROR, false)
    })
}
