//! Qualification through serve_admitted and the composed native caster, sharing the
//! existing fresh-admission PostgreSQL fixture and the existing LOOK framing helper.
use super::rune_use_bridge_fixtures::{Case, run_case};

use super::TestResult;
use super::fixture::run;
use super::item_look_fixtures::large_stack;

#[test]
fn rune_use_bridge_postgres_real_dispatch_casts_once_and_reloads_one_debit() -> TestResult {
    large_stack(scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        for case in [
            Case::Replay,
            Case::Protection,
            Case::TooFar,
            Case::NoCapability,
            Case::UnqualifiedSource,
            Case::StaleSession,
            Case::LostContent,
            Case::BeforeCommit,
            Case::AfterCommit,
            Case::StaleRevision,
        ] {
            run_case(admin.clone(), case)
                .await
                .map_err(|e| format!("{case:?}: {e}"))?;
        }
        Ok(())
    })
}
