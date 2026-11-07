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

fn run_named(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::item_cosmetics::run_stock_named_outfit(name);
    eprintln!(
        "POSTGRES_CASE_COMPLETED S06_SOURCE_OUTFIT {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
#[test]
fn s06_stock_corrected_dream_warden_source_choice() -> TestResult {
    run_named("Dream Warden")
}
#[test]
fn s06_stock_corrected_glooth_engineer_source_choice() -> TestResult {
    run_named("Glooth Engineer")
}
#[test]
fn s06_stock_corrected_jungle_explorer_source_choice() -> TestResult {
    run_named("Jungle Explorer")
}
#[test]
fn s06_stock_corrected_addonless_newly_wed_source_choice() -> TestResult {
    run_named("Wedding")
}
