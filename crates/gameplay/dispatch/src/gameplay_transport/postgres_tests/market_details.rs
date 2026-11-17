//! T124 runs installed migrations, fenced settlement writers and the real command route.
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use super::TestResult;
#[cfg(feature = "pg-tests-g4")]
#[test]
fn t124_market_completed_prices_and_catalogue_through_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| super::market::scenario(false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "T124 market case thread")??;
    Ok(())
}

#[cfg(feature = "pg-tests-g4")]
#[test]
fn t124_market_buy_sell_tables_have_independent_pages_and_bounded_books() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| super::market::scenario(true, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "T124 dual book case thread")??;
    Ok(())
}
