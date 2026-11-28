//! Composed native shared-or-personal entry. Sealed pocket setup, never SQL flags.
use super::l6_diprath_room_fixtures::{TRIGGER};

use super::TestResult;


fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::scenario(name, TRIGGER, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Diprath room scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_diprath_room_closed_six_switches_seven_signs() -> TestResult {
    run("l6-diprath-room-closed")
}
#[test]
fn l6_diprath_room_shared_seven_without_signs() -> TestResult {
    run("l6-diprath-room-shared")
}
#[test]
fn l6_diprath_room_personal_eight_without_switches() -> TestResult {
    run("l6-diprath-room-personal")
}

#[test]
fn l6_diprath_room_expiry_during_progress_write() -> TestResult {
    run("l6-diprath-room-expiry")
}

#[test]
fn l6_diprath_room_committed_step_recovers_after_expiry() -> TestResult {
    run("l6-diprath-room-recovery")
}
