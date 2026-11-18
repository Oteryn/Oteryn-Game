//! Literal native USE progresses only the exact ordered source sign. No SQL flags.
use super::l6_native_signs_fixtures::{start};

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
            super::quest_world_callbacks_fixtures::scenario(name, start().map_err(|e| e.to_string())?, false)
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native sign scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_native_signs_ordered() -> TestResult {
    run("l6-native-signs-ordered")
}
#[test]
fn l6_native_signs_out_of_order() -> TestResult {
    run("l6-native-signs-out-of-order")
}
#[test]
fn l6_native_signs_missing_middle() -> TestResult {
    run("l6-native-signs-missing-middle")
}
#[test]
fn l6_native_signs_repeat_and_replay() -> TestResult {
    run("l6-native-signs-repeat")
}
#[test]
fn l6_native_signs_unrelated_control() -> TestResult {
    run("l6-native-signs-control")
}
