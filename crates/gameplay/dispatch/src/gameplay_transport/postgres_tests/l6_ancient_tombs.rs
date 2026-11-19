//! L6 instrument progression through actual USE dispatch and the fenced Quest owner.
use super::l6_ancient_tombs_fixtures::{INSTRUMENTS};

use super::TestResult;


#[test]
fn l6_ancient_tombs_instruments_real_use_persists_order_and_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6_ancient_tombs_instruments");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks_fixtures::scenario(
                "l6-ancient-instruments",
                INSTRUMENTS[0].1,
                false,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "L6 instrument scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED l6_ancient_tombs_instruments duration={:?}",
        started.elapsed()
    );
    Ok(())
}
