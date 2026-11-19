//! Physical admission only: all source parts and the uncharged helmet retain real custody.
use super::l6_floor_profiles_fixtures::{TILE};

use super::TestResult;

#[test]
fn l6_floor_profiles_eight_exact_items_survive_custody_drop_and_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6_floor_profiles");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks_fixtures::scenario("l6-floor-profiles", TILE, false)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "floor profile scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED l6_floor_profiles duration={:?}",
        started.elapsed()
    );
    Ok(())
}
