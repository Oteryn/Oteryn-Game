//! Seven real native STEP flags, then the source all-flags gate. No SQL flag setup.
use super::l6_native_flames_fixtures::{GATE};

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
            super::quest_world_callbacks_fixtures::scenario(name, GATE, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native flame scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_native_flames_missing_last() -> TestResult {
    run("l6-native-flames-missing-last")
}
#[test]
fn l6_native_flames_missing_middle() -> TestResult {
    run("l6-native-flames-missing-middle")
}
#[test]
fn l6_native_flames_all_seven() -> TestResult {
    run("l6-native-flames-all")
}
#[test]
fn l6_native_flames_unordered() -> TestResult {
    run("l6-native-flames-unordered")
}
#[test]
fn l6_native_flames_repeat_and_replay() -> TestResult {
    run("l6-native-flames-repeat")
}
