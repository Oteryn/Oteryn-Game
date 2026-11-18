//! A complete, internally consistent receipt cannot grant an illegal Forge result.
use super::TestResult;

use super::item_forge_fixtures::support::{self, Case};

#[test]
fn forge_postgres_fabricated_complete_receipts_cannot_grant_illegal_tiers_or_custody() -> TestResult
{
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-integrity", || {
        support::scenario(Case::Integrity)
    })
}
