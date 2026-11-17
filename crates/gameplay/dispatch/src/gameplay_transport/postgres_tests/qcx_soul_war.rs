//! Global Soul War timers from TibiaWiki spoiler curid 91800, oldid 1213730.
use super::TestResult;

#[test]
fn malice_cage_expiry_is_fourteen_seconds_postgres() -> TestResult {
    super::srx3_source_death_timers::run_malice_source_death_timers(false, 14_000)
}

#[test]
fn megalomania_cooldown_is_sixty_eight_hours_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::srx3_megalomania_step::run_megalomania_native_step(Some(68 * 3600))?;
    eprintln!("QCX_MEGALOMANIA duration={:?}", started.elapsed());
    Ok(())
}
