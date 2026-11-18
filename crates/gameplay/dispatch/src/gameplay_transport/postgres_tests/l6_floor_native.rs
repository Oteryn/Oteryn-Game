//! Authored native altar counterpart: real drops through the existing floor crafting owner.
use super::l6_floor_native_fixtures::{ALTAR};

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
            super::quest_world_callbacks_fixtures::scenario(name, ALTAR, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "native altar scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_native_altar_assembly() -> TestResult {
    run("l6-floor-assembly-native")
}
#[test]
fn l6_native_altar_missing() -> TestResult {
    run("l6-floor-assembly-native-missing")
}
#[test]
fn l6_native_altar_unbound() -> TestResult {
    run("l6-floor-assembly-native-unbound")
}
#[test]
fn l6_native_altar_ruby_exact_one() -> TestResult {
    run("l6-floor-assembly-native-ruby-stack")
}
#[test]
fn l6_native_altar_ruby_unbound() -> TestResult {
    run("l6-floor-assembly-native-ruby-unbound")
}
