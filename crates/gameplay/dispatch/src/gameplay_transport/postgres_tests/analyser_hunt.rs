use super::party_membership_fixtures::{HuntCase, run_hunt};

#[test]
fn analyser_party_reset_through_route_clears_shared_measurements() -> super::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_hunt(admin, HuntCase::Reset)
}

#[test]
fn analyser_party_valuation_through_route_reprices_retained_counts() -> super::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_hunt(admin, HuntCase::Valuation)
}

#[test]
fn analyser_npc_mean_buy_price_through_route() -> super::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_hunt(admin, HuntCase::NpcPrices)
}
