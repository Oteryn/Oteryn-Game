//! Monotone room progress through the real PostgreSQL quest transaction owner.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

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
        .map_err(|_| "room raise scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_room_raise_lower() -> TestResult {
    run("l6-room-raise-lower")
}
#[test]
fn l6_room_raise_equal() -> TestResult {
    run("l6-room-raise-equal")
}
#[test]
fn l6_room_raise_higher() -> TestResult {
    run("l6-room-raise-higher")
}
#[test]
fn l6_room_raise_refusal() -> TestResult {
    run("l6-room-raise-refusal")
}
#[test]
fn l6_room_raise_extreme() -> TestResult {
    run("l6-room-raise-extreme")
}
