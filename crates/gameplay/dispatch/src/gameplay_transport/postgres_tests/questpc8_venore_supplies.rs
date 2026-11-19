//! Venore job supplies retain exact source custody facts, without fabricated charges.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_venore_supplies_mint_and_carry_without_invented_charges() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-venore-supplies", || {
        super::quest_world_callbacks_fixtures::scenario(
            "authored-venore-supplies",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_venore_supplies_mint_and_carry_without_invented_charges duration={:?}",
        started.elapsed()
    );
    Ok(())
}
