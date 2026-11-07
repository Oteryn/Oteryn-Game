//! Stock outfit item acquisition must bind the actual source grant catalogue.
use super::TestResult;
#[test]
fn s06_stock_random_outfit_item_grant_replay_and_selection() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::item_cosmetics::run_stock_random_outfit();
    eprintln!(
        "POSTGRES_CASE_COMPLETED S06_STOCK_RANDOM_OUTFIT duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
