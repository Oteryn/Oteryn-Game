//! Intense Healing targeting through real native USE admission.
use super::TestResult;
use super::fixture::run;
use super::item_look_fixtures::large_stack;
use super::rune_use_bridge_fixtures::{Case, run_case};

#[test]
fn healing_parity_matrix_rune_3152_rejects_own_summon_without_spending() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            run_case(admin, Case::IntenseHealingOwnSummon).await?;
            eprintln!("PG_EXECUTED rune_3152 TargetIllegal with unchanged summon health and charges");
            Ok(())
        })
    })
}
