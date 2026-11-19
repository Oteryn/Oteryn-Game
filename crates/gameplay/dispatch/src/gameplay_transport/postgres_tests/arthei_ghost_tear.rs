use super::TestResult;
use super::arthei_ghost_tear_fixtures::{NAME, STATUE};

#[test]
fn arthei_statue_consumes_exact_tear_and_moves_two_players_to_spawned_room() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, STATUE, false)
    })
}
