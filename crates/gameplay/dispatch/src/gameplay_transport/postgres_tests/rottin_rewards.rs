//! Exercise the authored reward owner through live Rottin TALK and PostgreSQL.
use super::TestResult;
use super::creature_push_fixtures::scenario;

#[test]
fn rottin_lucky_loop_talk_mints_one_weighted_bundle_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_weighted_reward", false, None))?
        .join()
        .map_err(|_| "Rottin reward PostgreSQL thread panicked")?
        .map_err(Into::into)
}

#[test]
fn rottin_lucky_loop_talk_refuses_six_charms_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_weighted_short", false, None))?
        .join()
        .map_err(|_| "Rottin reward PostgreSQL thread panicked")?
        .map_err(Into::into)
}

#[test]
fn rottin_lucky_loop_talk_mints_after_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_weighted_repeat", false, None))?
        .join()
        .map_err(|_| "Rottin repeat reward PostgreSQL thread panicked")?
        .map_err(Into::into)
}

#[test]
fn rottin_lucky_charms_talk_starts_rabbit_task_and_grants_ribbons_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_lucky_charms", false, None))?
        .join()
        .map_err(|_| "Rottin Lucky Charms PostgreSQL thread panicked")?
        .map_err(Into::into)
}
