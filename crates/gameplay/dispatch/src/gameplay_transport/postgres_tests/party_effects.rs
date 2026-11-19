//! Real party authority and admitted router, reused by the existing membership scenario.
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn party_effects_real_dispatch_postgres_membership_and_shared_eligibility() -> TestResult {
    let started = std::time::Instant::now();
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::party_membership_fixtures::run(true)?;
    super::party_buffs_fixtures::run_party_buffs(None)?;
    for extra in [
        ("train_party", "knight"),
        ("enchant_party", "sorcerer"),
        ("heal_party", "druid"),
    ] {
        super::party_buffs_fixtures::run_party_buffs(Some(extra))?;
    }
    eprintln!(
        "POSTGRES_EXECUTED case=party_effects elapsed={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
