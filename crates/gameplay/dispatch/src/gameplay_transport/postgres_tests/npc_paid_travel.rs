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
fn npc_paid_travel_confirmation_retries_holder_and_records_arrival() -> TestResult {
    confirmation_fence(super::npc_paid_travel_fixtures::FenceFault::Busy)
}

#[test]
fn npc_paid_travel_confirmation_reports_nontransient_fence_error() -> TestResult {
    confirmation_fence(super::npc_paid_travel_fixtures::FenceFault::MissingRelation)
}

fn confirmation_fence(fault: super::npc_paid_travel_fixtures::FenceFault) -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || super::npc_paid_travel_fixtures::scenario_with_fence_fault(fault)
            .map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "travel fence thread panicked")??;
    Ok(())
}
