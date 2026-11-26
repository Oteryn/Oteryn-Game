//! P18 item operations through command 181, the composed owner and native PostgreSQL rows.

use super::TestResult;

use super::item_forge_fixtures::support::{self, Case};

use oteryn_protocol_oteryn::item_forge::Operation;


#[test]
fn forge_postgres_fusion_success_failure_protection_bonuses_and_replay() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-fusion", || {
        support::scenario(Case::Items(Operation::Fusion))
    })
}

#[test]
fn forge_postgres_convergence_fusion_preserves_insufficient_gold_refusal_and_replay() -> TestResult
{
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-convergence-fusion", || {
        support::scenario(Case::Items(Operation::ConvergenceFusion))
    })
}

#[test]
fn forge_transfer_prices_by_donor_tier_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-transfer", || {
        support::scenario(Case::Items(Operation::Transfer))
    })
}

#[test]
fn forge_postgres_convergence_transfer_accepts_tier_one_and_replays_consumed_source() -> TestResult
{
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-convergence-transfer", || {
        support::scenario(Case::Items(Operation::ConvergenceTransfer))
    })
}

#[test]
fn forge_postgres_insufficient_cores_refusal_survives_refill_and_fresh_occurrence_succeeds()
-> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-core-refusal", || {
        support::scenario(Case::CoreRefusal)
    })
}

#[test]
fn forge_postgres_mixed_physical_coins_and_bank_fee_replays_exactly_once() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-mixed-fee", || {
        support::scenario(Case::MixedFee)
    })
}
