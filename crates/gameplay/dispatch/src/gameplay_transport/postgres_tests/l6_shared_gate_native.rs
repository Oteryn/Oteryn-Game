//! Native counterpart threshold transports, real native USE and STEP, no synthetic IDs.
use super::l6_shared_gate_native_fixtures::{position};

use super::TestResult;

#[test]
fn l6_native_gate_closed_50135() -> TestResult {
    run("l6-tomb-gate-native-closed-50135")
}
#[test]
fn l6_native_gate_open_50135() -> TestResult {
    run("l6-tomb-gate-native-open-50135")
}
#[test]
fn l6_native_gate_closed_50137() -> TestResult {
    run("l6-tomb-gate-native-closed-50137")
}
#[test]
fn l6_native_gate_open_50137() -> TestResult {
    run("l6-tomb-gate-native-open-50137")
}
#[test]
fn l6_native_gate_closed_50138() -> TestResult {
    run("l6-tomb-gate-native-closed-50138")
}
#[test]
fn l6_native_gate_open_50138() -> TestResult {
    run("l6-tomb-gate-native-open-50138")
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
        .map_err(|_| "native gate fixture panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
