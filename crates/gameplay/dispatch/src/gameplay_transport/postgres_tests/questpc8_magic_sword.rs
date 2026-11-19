//! Chosen archival vault programme uses native chambers and the existing quest/MINT owners.
use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_magic_sword_two_and_four_native_demons_both_caches_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-magic-sword", || {
        quest_world_callbacks::scenario(
            "authored-magic-sword",
            Position {
                x: 32575,
                y: 32197,
                floor: -14,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_magic_sword_two_and_four_native_demons_both_caches_once duration={:?}",
        started.elapsed()
    );
    Ok(())
}
