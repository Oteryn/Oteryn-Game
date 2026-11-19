//! Original shared World switch deadlines drive the existing Channel decay owner.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn l6_switch_timer_expired_12121_restores_source_lever() -> TestResult {
    run("l6-tomb-switch-timer-expired-12121")
}
#[test]
fn l6_switch_timer_expired_12122_restores_source_lever() -> TestResult {
    run("l6-tomb-switch-timer-expired-12122")
}
#[test]
fn l6_switch_timer_expired_12123_restores_source_lever() -> TestResult {
    run("l6-tomb-switch-timer-expired-12123")
}
#[test]
fn l6_switch_timer_future_does_not_restore_early() -> TestResult {
    run("l6-tomb-switch-timer-fresh-12121")
}
#[test]
fn l6_switch_timer_repeated_cycles_keep_source_policy() -> TestResult {
    run("l6-tomb-switch-timer-repeat-12121")
}
#[test]
fn l6_switch_timer_future_occurrence_is_rejected_by_pg() -> TestResult {
    run("l6-tomb-switch-timer-forged-12121")
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
        .map_err(|_| "switch timer fixture panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
