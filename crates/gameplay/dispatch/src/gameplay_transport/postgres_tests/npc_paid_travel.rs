//! TRAVEL: production NPC_TALK router, runtime movement owner and runtime-role PostgreSQL.
use super::npc_paid_travel_fixtures::{scenario};

use super::TestResult;

#[test]
fn npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "paid travel thread panicked")??;
    Ok(())
}

#[test]
fn postman_tour_passages_travel_without_credit_under_an_unrecognized_pin() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("postman-tour-foreign-pin", || {
        super::npc_paid_travel_fixtures::scenario_with_tour_pin(false, true)
    })
}
