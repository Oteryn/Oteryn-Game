//! PREM-2b+PREM-5 through command 7's real composed dispatch and runtime-role PostgreSQL.
use super::promotion_npc_fixtures::{run_source_promotion_scenario};

use super::TestResult;

#[test]
fn promotion_npc_postgres_dispatch_purchase_replay_bank_login_and_death() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| run_source_promotion_scenario("eremo").map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "promotion PostgreSQL thread panicked")??;
    Ok(())
}
