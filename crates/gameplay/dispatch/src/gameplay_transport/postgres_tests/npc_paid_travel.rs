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
