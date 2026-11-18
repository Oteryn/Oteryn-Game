//! Exact shared threshold gates after real physical switch USE commands.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn l6_shared_gate_closed_50135_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-closed-50135")
}
#[test]
fn l6_shared_gate_open_50135_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-open-50135")
}
#[test]
fn l6_shared_gate_closed_50137_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-closed-50137")
}
#[test]
fn l6_shared_gate_open_50137_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-open-50137")
}
#[test]
fn l6_shared_gate_closed_50138_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-closed-50138")
}
#[test]
fn l6_shared_gate_open_50138_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-open-50138")
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
        .map_err(|_| "shared gate fixture panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
