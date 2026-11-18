//! Exact retained B3 switch counterparts, with unrelated native lever controls.
use super::l6_shared_switch_native_fixtures::{position};

use super::TestResult;

#[test]
fn l6_native_switch_present_12121() -> TestResult {
    run("l6-tomb-switch-native-present-12121")
}
#[test]
fn l6_native_switch_control_12121() -> TestResult {
    run("l6-tomb-switch-native-control-12121")
}
#[test]
fn l6_native_switch_present_12122() -> TestResult {
    run("l6-tomb-switch-native-present-12122")
}
#[test]
fn l6_native_switch_control_12122() -> TestResult {
    run("l6-tomb-switch-native-control-12122")
}
#[test]
fn l6_native_switch_present_12123() -> TestResult {
    run("l6-tomb-switch-native-present-12123")
}
#[test]
fn l6_native_switch_control_12123() -> TestResult {
    run("l6-tomb-switch-native-control-12123")
}
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let trigger = position(name)?;
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::scenario(name, trigger, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native switch fixture panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
