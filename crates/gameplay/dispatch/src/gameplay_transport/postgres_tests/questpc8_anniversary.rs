//! Chosen seasonal route: real native deaths, actual corpse collection and exact owned input.

use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_anniversary_thirty_native_rag_drops_five_pinatas_and_once_only_backpack() -> TestResult
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-anniversary", || {
        quest_world_callbacks::scenario(
            "authored-anniversary",
            Position {
                x: 32359,
                y: 32234,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_anniversary_thirty_native_rag_drops_five_pinatas_and_once_only_backpack duration={:?}",
        started.elapsed()
    );
    Ok(())
}
