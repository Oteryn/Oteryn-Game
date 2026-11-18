//! Chosen smith cache uses the existing death and multi-output reward owners.
use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_smith_cache_native_guardian_and_atomic_outputs() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-smith-cache", || {
        quest_world_callbacks::scenario(
            "authored-smith-cache",
            Position {
                x: 32776,
                y: 32254,
                floor: -8,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_smith_cache_native_guardian_and_atomic_outputs duration={:?}",
        started.elapsed()
    );
    Ok(())
}
