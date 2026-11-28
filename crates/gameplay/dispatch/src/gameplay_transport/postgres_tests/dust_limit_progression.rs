//! Global Dust Limit progression through command 181 and the native ledger.
use super::TestResult;
use super::item_forge_fixtures::support::{self, Case};

#[test]
fn dust_limit_purchase_follows_global_progression_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("global-dust-limit", || {
        for limit in [100, 4_580] {
            support::scenario(Case::DustProgression(limit))?;
        }
        Ok(())
    })
}

#[path = "dust_limit_progression/legacy_receipt.rs"]
mod legacy_receipt;
