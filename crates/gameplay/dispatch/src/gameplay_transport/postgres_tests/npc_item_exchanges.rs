//! Atomic recipe transactions, driven through the existing fenced NPC writer.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;


#[test]
fn npc_item_exchanges_consume_exact_recipes_with_one_receipt_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("npc-item-exchanges", || {
        super::quest_world_callbacks_fixtures::scenario(
            "npc-item-exchanges",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "NPC_EXCHANGE POSTGRES_CASE_COMPLETED three_source_recipes duration={:?}",
        started.elapsed()
    );
    Ok(())
}
