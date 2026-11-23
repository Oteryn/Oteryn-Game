use super::TestResult;
use super::hireling_lifecycle_fixtures::{Case, run_case_with_admin};

#[test]
fn owner_use_wakes_both_bed_halves_once_postgres() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = run_case_with_admin(Case::BedOwnerWake, admin);
    eprintln!(
        "PG_MARKER S07 bed-owner-wake success={} duration={:.3}s",
        result.is_ok(),
        started.elapsed().as_secs_f64()
    );
    result
}
