//! Registered M19 callbacks through real wire USE and PostgreSQL custody.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn qready1_bewitched_daily_contribution_real_dispatch() -> TestResult {
    run(
        "qready1-bewitched-contribution",
        Position {
            x: 32769,
            y: 32026,
            floor: -8,
        },
    )
}
#[test]
fn qready1_raven_herb_interval_real_dispatch() -> TestResult {
    run(
        "qready1-threatened-raven-herb",
        Position {
            x: 33497,
            y: 32196,
            floor: -7,
        },
    )
}
fn run(name: &'static str, position: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::scenario(name, position, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "M19 real-dispatch PG thread failed")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
