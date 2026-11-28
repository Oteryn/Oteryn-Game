//! Chosen Kormarak cache uses the existing death and multi-output reward owners.
use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_kormarak_native_guardian_body_and_four_outputs() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-kormarak", || {
        quest_world_callbacks::scenario(
            "authored-kormarak",
            Position {
                x: 32357,
                y: 32171,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_kormarak_native_guardian_body_and_four_outputs duration={:?}",
        started.elapsed()
    );
    Ok(())
}
