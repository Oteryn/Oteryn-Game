//! Source construction recipes require complete atomic carried ingredients.
use super::TestResult;
fn execute(ground: bool, missing: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::item_cosmetics::run_stock_glider(ground, missing);
    eprintln!(
        "POSTGRES_CASE_COMPLETED S06_GLIDER ground={ground} missing={missing} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
#[test]
fn s06_glider_seven_carried_pieces_are_atomic_and_replayed() -> TestResult {
    execute(false, false)
}
#[test]
fn s06_glider_missing_one_piece_never_spends() -> TestResult {
    execute(false, true)
}
#[test]
fn s06_glider_ground_trigger_preserves_source_and_consumes_carried_recipe() -> TestResult {
    execute(true, false)
}
