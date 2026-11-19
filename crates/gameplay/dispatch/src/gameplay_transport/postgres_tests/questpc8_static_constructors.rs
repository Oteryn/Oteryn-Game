//! First-party plain constructors use the existing MINT and custody owners.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_static_constructors_mint_and_carry_four_plain_items_without_timers() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-static-constructors", || {
        super::quest_world_callbacks_fixtures::scenario(
            "authored-static-constructors",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_static_constructors_mint_and_carry_four_plain_items_without_timers duration={:?}",
        started.elapsed()
    );
    Ok(())
}
