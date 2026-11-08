//! Source acquisition must work with the stock artifact, without synthetic mount definitions.
use super::TestResult;
use super::mount_taming::{Cases, run_stock_case};
#[test]
fn s06_stock_native_taming_outcomes_and_refusals() -> TestResult {
    run_stock_case(Cases::StockOutcomes)
}
#[test]
fn s06_stock_native_taming_all_item_pairs() -> TestResult {
    run_stock_case(Cases::StockItems)
}
#[test]
fn s06_stock_native_music_box_pairs() -> TestResult {
    run_stock_case(Cases::StockMusicBox)
}
