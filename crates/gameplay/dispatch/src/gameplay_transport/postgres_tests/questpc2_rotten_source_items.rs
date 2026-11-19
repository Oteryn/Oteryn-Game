//! Source Item admission through real mint and carried custody owners, without use effects.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn rotten_source_items_materialize_into_exact_character_custody() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("ROTTEN_SOURCE_ITEMS_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc2-rotten-source-items", || {
        super::quest_world_callbacks_fixtures::scenario(
            "questpc2-rotten-source-items",
            Position { x: 33294, y: 31677, floor: -15 },
            false,
        )
    })?;
    eprintln!("ROTTEN_SOURCE_ITEMS POSTGRES_CASE_COMPLETED {:?}", started.elapsed());
    Ok(())
}
