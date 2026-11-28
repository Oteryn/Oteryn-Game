use super::{
    TestResult,
    koshei_the_deathless_journey_fixtures::{CRYPT_GATE, NAME},
};

#[test]
fn koshei_the_deathless_full_journey() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, CRYPT_GATE, false)
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
