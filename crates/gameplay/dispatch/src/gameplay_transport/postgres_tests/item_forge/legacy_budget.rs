//! Existing DustLimit callers and command181 share one bounded retry allowance.
use super::*;

#[test]
fn forge_postgres_legacy_retry_exhaustion_cannot_reset_through_command181() -> TestResult {
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("forge-legacy-budget", || {
        support::scenario(Case::LegacyBudget)
    })
}
