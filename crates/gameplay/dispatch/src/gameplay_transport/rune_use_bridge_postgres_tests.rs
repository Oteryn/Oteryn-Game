//! Qualification through serve_admitted and the composed native caster, sharing the
//! existing fresh-admission PostgreSQL fixture and the existing LOOK framing helper.
use super::rune_use_bridge_fixtures::{Case, run_case};

use super::TestResult;
use super::fixture::run;
use super::item_look_fixtures::large_stack;

#[test]
fn rune_use_bridge_postgres_after_commit_reloads_one_debit() -> TestResult {
    large_stack(|| scenario(Case::AfterCommit))
}

#[test]
fn rune_use_bridge_postgres_before_commit_reconciles_the_cast() -> TestResult {
    large_stack(|| scenario(Case::BeforeCommit))
}

#[test]
fn closed_sudden_death_shortcut_casts_and_debits_once() -> TestResult {
    large_stack(|| scenario(Case::ClosedSuddenDeath))
}

#[test]
fn rune_use_bridge_postgres_lost_content_disconnects() -> TestResult {
    large_stack(|| scenario(Case::LostContent))
}

#[test]
fn rune_use_bridge_postgres_missing_capability_refuses_the_cast() -> TestResult {
    large_stack(|| scenario(Case::NoCapability))
}

#[test]
fn rune_use_bridge_postgres_out_of_range_refuses_the_cast() -> TestResult {
    large_stack(|| scenario(Case::TooFar))
}

#[test]
fn rune_use_bridge_postgres_protection_refuses_the_cast() -> TestResult {
    large_stack(|| scenario(Case::Protection))
}

#[test]
fn rune_use_bridge_postgres_replay_casts_once() -> TestResult {
    large_stack(|| scenario(Case::Replay))
}

#[test]
fn rune_use_bridge_postgres_stale_revision_refuses_reconciliation() -> TestResult {
    large_stack(|| scenario(Case::StaleRevision))
}

#[test]
fn rune_use_bridge_postgres_stale_session_disconnects() -> TestResult {
    large_stack(|| scenario(Case::StaleSession))
}

#[test]
fn rune_use_bridge_postgres_unqualified_source_refuses_the_cast() -> TestResult {
    large_stack(|| scenario(Case::UnqualifiedSource))
}

fn scenario(case: Case) -> TestResult {
    run(async move |admin| {
        run_case(admin, case)
            .await
            .map_err(|e| format!("{case:?}: {e}"))?;
        Ok(())
    })
}
