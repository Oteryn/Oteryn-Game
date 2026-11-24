//! PARTY-SKULL-1 through the actual admitted command router and visibility cadence.
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn party_skull_real_dispatch_postgres_end_to_end() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::party_membership_fixtures::run(true)?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED party_skull elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn party_skull_pvp_real_dispatch_attribution_counterfactual() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::item_look_fixtures::large_stack(|| super::pvp_live_fixtures::scenario(14))?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED party_skull_pvp elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}
