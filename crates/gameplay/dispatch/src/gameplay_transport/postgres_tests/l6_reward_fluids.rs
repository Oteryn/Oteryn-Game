//! Shared reward mechanism proof; this fixture is not a source barrel binding.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn l6_reward_container_preserves_mead_and_exact_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6_reward_fluids");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks_fixtures::scenario(
                "l6-reward-fluids",
                Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                false,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .unwrap_or_else(|_| Err("filled reward fixture panicked".into()));
    eprintln!(
        "POSTGRES_CASE_COMPLETED l6_reward_fluids duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
