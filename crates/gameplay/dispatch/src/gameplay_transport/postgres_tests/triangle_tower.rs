//! Source-literal Triangle Tower lever, including its different closing wall.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn triangle_tower_source_wall_cycle_over_use_wire() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks_fixtures::scenario(
                "triangle-tower-wall",
                Position {
                    x: 32573,
                    y: 32121,
                    floor: -7,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Triangle Tower wire panic")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED triangle_tower_wall duration={:?}",
        started.elapsed()
    );
    Ok(())
}
