//! Shared physical source pickups: no personal quest claim substitutes for map custody.
use super::l6_world_pickups_fixtures::{RUBY};

use super::TestResult;

#[test]
fn l6_small_ruby_native_pickup_replay_and_shared_custody() -> TestResult {
    run_case("l6-small-ruby")
}

#[test]
fn l6_small_ruby_actual_reset_replenishes_and_refuses_old_epoch() -> TestResult {
    run_case("l6-small-ruby-reset")
}

fn run_case(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = (|| -> TestResult {
        std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn(move || {
                super::quest_world_callbacks_fixtures::scenario(name, RUBY, false)
                    .map_err(|error| error.to_string())
            })?
            .join()
            .map_err(|_| "Small Ruby source pickup panicked")??;
        Ok(())
    })();
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
