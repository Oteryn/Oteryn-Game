//! Chosen two-cache potion journey, Premium gates and consolidated gold.

use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_mastermind_two_native_caches_premium_and_exact_201_gold() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-mastermind", || {
        quest_world_callbacks::scenario(
            "authored-mastermind",
            Position {
                x: 32216,
                y: 31170,
                floor: -11,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_mastermind_two_native_caches_premium_and_exact_201_gold duration={:?}",
        started.elapsed()
    );
    Ok(())
}
