//! Drive a real retained hammer USE and the resulting Disrepair hand-in.
use super::TestResult;
use super::creature_push_fixtures::scenario;

#[test]
fn rottin_hammer_use_satisfies_disrepair_turn_in_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_repairs", false, None))?
        .join()
        .map_err(|_| "Disrepair PostgreSQL thread panicked")?
        .map_err(Into::into)
}

#[test]
fn rottin_hammer_use_satisfies_handyman_turn_in_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_handyman", false, None))?
        .join()
        .map_err(|_| "Handyman PostgreSQL thread panicked")?
        .map_err(Into::into)
}
