//! PARTY-SKULL-1 through the actual admitted command router and visibility cadence.
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn party_skull_real_dispatch_postgres_end_to_end() -> TestResult {
    super::party_membership_fixtures::run(true)
}

#[test]
fn party_skull_pvp_real_dispatch_attribution_counterfactual() -> TestResult {
    super::item_look_fixtures::large_stack(|| super::pvp_live_fixtures::scenario(14))
}
