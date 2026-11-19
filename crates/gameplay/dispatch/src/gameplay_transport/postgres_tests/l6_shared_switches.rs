//! Shared source switch activation uses literal USE, without invented Quest progress.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn l6_shared_switch_12121_activation_is_durable_and_replay_safe() -> TestResult {
    run("l6-tomb-switch-12121")
}
#[test]
fn l6_shared_switch_12122_activation_is_durable_and_replay_safe() -> TestResult {
    run("l6-tomb-switch-12122")
}
#[test]
fn l6_shared_switch_12123_activation_is_durable_and_replay_safe() -> TestResult {
    run("l6-tomb-switch-12123")
}
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::scenario(
                name,
                Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "shared switch fixture panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
