//! Native room proof and literal USE of the conservatively selected eastern barrel.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

use super::l6_mead_fixtures::NAME;
#[test]
fn l6_mead_native_barrel_requires_mission_and_mints_complete_filled_bag() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {NAME}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks_fixtures::scenario(
                NAME,
                Position {
                    x: 32215,
                    y: 31060,
                    floor: -10,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .unwrap_or_else(|_| Err("Mead fixture panicked".into()));
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
