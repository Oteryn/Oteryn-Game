use super::{TestResult, market_fixtures::scenario_with_price_lookup};

#[test]
fn market_price_lookup_works_away_from_depot_and_keeps_offer_gate() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_with_price_lookup().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "market price lookup scenario panicked")??;
    Ok(())
}
