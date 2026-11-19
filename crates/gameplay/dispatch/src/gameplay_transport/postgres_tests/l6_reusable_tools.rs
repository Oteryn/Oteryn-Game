//! Reusable tools must keep their physical identity when a quest action commits.
use super::l6_reusable_tools_fixtures::{TRIGGER};

use super::TestResult;

#[test]
fn l6_reusable_tool_preserves_instance_and_refuses_missing_tool() -> TestResult {
    run_case("l6-reusable-tool")
}

#[test]
fn l6_reusable_multitool_preserves_instance_and_refuses_missing_tool() -> TestResult {
    run_case("l6-reusable-tool-multitool")
}

fn run_case(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::scenario(name, TRIGGER, false)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "reusable tool scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
