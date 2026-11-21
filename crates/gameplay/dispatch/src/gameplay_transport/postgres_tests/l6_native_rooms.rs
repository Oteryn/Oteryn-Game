//! Exact native interaction pockets; no claim of traversing the intervening maze.
use super::l6_native_rooms_fixtures::{spec};

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
            super::quest_world_callbacks_fixtures::scenario(
                name,
                spec(name).map_err(|e| e.to_string())?.trigger,
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native room scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_native_room_morguthis_entry() -> TestResult {
    run("l6-native-room-morguthis-entry")
}
#[test]
fn l6_native_room_omruc_specific() -> TestResult {
    run("l6-native-room-omruc-specific")
}
#[test]
fn l6_native_room_omruc_room() -> TestResult {
    run("l6-native-room-omruc-room")
}
#[test]
fn l6_native_room_rahemos_levers() -> TestResult {
    run("l6-native-room-rahemos-levers")
}
#[test]
fn l6_native_room_rahemos_room() -> TestResult {
    run("l6-native-room-rahemos-room")
}

#[test]
fn l6_native_room_vashresamun_specific() -> TestResult {
    run("l6-native-room-vashresamun-specific")
}
#[test]
fn l6_native_room_vashresamun_specific_preserve() -> TestResult {
    run("l6-native-room-vashresamun-specific-preserve")
}
#[test]
fn l6_native_room_vashresamun_room() -> TestResult {
    run("l6-native-room-vashresamun-room")
}
