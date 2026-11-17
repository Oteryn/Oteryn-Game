//! Elemental overlord production lethal queue and durable quest receipt regression.
use super::TestResult;

#[test]
fn overlordcr_each_overlord_sets_donor_flag_and_replay_is_idempotent_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    for creature in ["earth_overlord", "energy_overlord", "fire_overlord", "ice_overlord"] {
        super::srvenc_source_deaths::scenario("overlordcr", creature)?;
    }
    Ok(())
}
