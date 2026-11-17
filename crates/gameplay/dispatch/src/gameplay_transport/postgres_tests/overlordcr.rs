//! Elemental overlord production lethal queue and durable quest receipt regression.
use super::TestResult;

#[test]
fn overlordcr_each_overlord_sets_donor_flag_and_replay_is_idempotent_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    for creature in ["earth_overlord", "energy_overlord", "fire_overlord", "ice_overlord"] {
        super::srvenc_source_deaths::scenario("overlordcr", creature)?;
    }
    Ok(())
}

#[test]
fn overlordcr_non_overlord_does_not_set_flag_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::srvenc_source_deaths::scenario("overlordcr_non_overlord", "goblin_leader")
}
