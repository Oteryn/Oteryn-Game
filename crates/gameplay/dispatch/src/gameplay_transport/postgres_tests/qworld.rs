//! Shared world-change occurrence through reset and the real admitted dispatch path.
use super::*;
#[path = "qworld/consumers.rs"]
pub(super) mod consumers;
#[path = "qworld/rules.rs"]
pub(super) mod rules;

#[test]
fn qworld_postgres_occurrence_dispatch_and_restart() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let world = crate::foundation::WorldId::decode(&id(42))
                .map_err(debug)
                .map_err(|e| e.to_string())?;
            let active_epoch = (1..=64)
                .find(|epoch| {
                    let day = oteryn_gameplay_world_change::daily::roll_calendar(
                        world.as_bytes(),
                        *epoch,
                        1,
                        1,
                    );
                    day.their_masters_voice && day.fury_gates.is_some()
                })
                .ok_or("no active deterministic occurrence in qualification bound")?;
            super::world_event::scenario_with_consumers(active_epoch, true)
                .map_err(|error| error.to_string())?;
            Ok::<_, String>(())
        })?
        .join()
        .map_err(|_| "QWORLD scenario thread failed")??;
    eprintln!(
        "QWORLD_PG_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
