//! Exercise the Lucky Charm recipe after starting the rabbit hunt through live Rottin dialogue.
use super::TestResult;
use super::creature_push_fixtures::scenario;

#[test]
fn rottin_rabbit_task_crafts_lucky_charm_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_lucky_charm_craft", false, None))?
        .join()
        .map_err(|_| "Lucky Charm craft PostgreSQL thread panicked")?
        .map_err(Into::into)
}
