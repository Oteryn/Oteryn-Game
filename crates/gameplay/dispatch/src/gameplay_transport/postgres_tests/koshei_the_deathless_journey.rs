use super::{
    TestResult,
    koshei_the_deathless_journey_fixtures::{CRYPT_GATE, KICK, LOSS, NAME, SPATIAL},
    quest_world_callbacks_fixtures::use_with_commit_probe,
};

#[test]
fn koshei_the_deathless_full_journey() -> TestResult {
    journey(NAME)
}

#[test]
fn koshei_spatial_corpse_on_first_visit() -> TestResult {
    journey(SPATIAL)
}

#[test]
fn koshei_committed_gate_staff_retirement() -> TestResult {
    journey(KICK)
}

#[test]
fn koshei_cancelled_gate_grace_expiry() -> TestResult {
    journey(LOSS)
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

#[tokio::test(start_paused = true)]
async fn late_commit_probe_returns_the_observer_timeout() {
    use std::time::Duration;
    let pause = (tokio::sync::Notify::new(), tokio::sync::Notify::new());
    let writer = async {
        tokio::time::sleep(Duration::from_secs(6)).await;
        pause.0.notify_one();
        pause.1.notified().await;
        Ok(())
    };
    let mut inspected = false;
    let command = use_with_commit_probe(writer, &pause, || inspected = true);
    let result = tokio::time::timeout(Duration::from_secs(7), command).await;
    assert!(matches!(result, Ok(Err(error)) if error.is::<tokio::time::error::Elapsed>()));
    assert!(!inspected);
}
