//! Crystal 201189f8 moviment_trap.lua: a real creature push delivers merchant StepIn.
use super::TestResult;
use super::creature_push_fixtures::scenario;

#[test]
fn rottin_merchant_step_in_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_bushwhacking", false, Some(12187)))?
        .join()
        .map_err(|_| "Bushwhacking PostgreSQL thread panicked")?
        .map_err(Into::into)
}

#[test]
fn rottin_merchant_step_loot_and_mission_turn_in_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_merchant_loot", false, Some(12187)))?
        .join()
        .map_err(|_| "Bushwhacking USE PostgreSQL thread panicked")?
        .map_err(Into::into)
}

#[test]
fn rottin_merchant_step_final_net_starts_scene_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_merchant_wave", false, Some(12187)))?
        .join()
        .map_err(|_| "Bushwhacking scene PostgreSQL thread panicked")?
        .map_err(Into::into)
}

#[test]
fn rottin_merchant_wave_skips_a_blocked_spawn_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_merchant_wave_blocked", false, Some(12187)))?
        .join()
        .map_err(|_| "Bushwhacking scene PostgreSQL thread panicked")?
        .map_err(Into::into)
}
