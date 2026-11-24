//! T124 runs installed migrations, fenced settlement writers and the real command route.
use super::TestResult;
#[test]
fn t124_market_completed_prices_and_catalogue_through_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| super::market_fixtures::scenario(false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "T124 market case thread")??;
    Ok(())
}

#[test]
fn t124_market_buy_sell_tables_have_independent_pages_and_bounded_books() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| super::market_fixtures::scenario(true, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "T124 dual book case thread")??;
    Ok(())
}

#[test]
fn market_catalogue_filters_by_forge_tier_through_dispatch() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("market-catalogue-tier", || {
        use super::market_fixtures::{scenario_with_catalogue_extension, CatalogueExtension};
        scenario_with_catalogue_extension(CatalogueExtension::Tier)
    })
}

#[test]
fn market_catalogue_projects_pinned_artwork_through_dispatch() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("market-catalogue-artwork", || {
        use super::market_fixtures::{scenario_with_catalogue_extension, CatalogueExtension};
        scenario_with_catalogue_extension(CatalogueExtension::Artwork)
    })
}
