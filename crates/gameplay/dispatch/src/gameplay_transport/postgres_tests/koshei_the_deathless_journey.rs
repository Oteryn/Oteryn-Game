use super::{
    TestResult,
    koshei_the_deathless_journey_fixtures::{CRYPT_GATE, NAME, SPATIAL},
};

#[test]
fn koshei_the_deathless_full_journey() -> TestResult {
    journey(NAME)
}

#[test]
fn koshei_spatial_corpse_on_first_visit() -> TestResult {
    journey(SPATIAL)
}

fn journey(name: &'static str) -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, CRYPT_GATE, false)
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
