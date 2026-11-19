//! Retained pending attempts across content cutover, and unreceipted SQL refusals.
use super::*;

#[test]
fn forge_postgres_pending_attempt_retains_revision_change_refusal_without_spending() -> TestResult {
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("forge-revision", || {
        support::scenario(Case::RevisionChanged)
    })
}

#[test]
fn forge_postgres_sql_rejects_unreceipted_tier_and_core_changes() -> TestResult {
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("forge-sql-guards", || {
        support::scenario(Case::SqlGuards)
    })
}

#[test]
fn forge_postgres_pending_dust_limit_never_requotes_after_another_paid_upgrade() -> TestResult {
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("forge-dust-stage", || {
        support::scenario(Case::DustStageChanged)
    })
}
