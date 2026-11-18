//! Incomplete saves retain their original pin through the real lethal/XP writer.
use super::TestResult;

#[test]
fn native_death_settles_incomplete_predecessor_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::native_objective_predecessor(false)
}

#[test]
fn native_death_replay_settles_incomplete_predecessor_once_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::native_objective_predecessor(true)
}
