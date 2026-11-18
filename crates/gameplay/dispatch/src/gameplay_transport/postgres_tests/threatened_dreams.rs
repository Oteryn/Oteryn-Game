//! Source NPC decisions through the actual dispatch trait and fenced QuestState writer.
//! The test does not seed successful quest flags or call a pure decision in place of dispatch.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn threatened_dreams_real_npc_source_start_fences_topics_and_recovers_lost_ack() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_DREAMS_NPC_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("threatened-dreams-npc", || {
        super::quest_world_callbacks_fixtures::scenario(
            "threatened-dreams-npc",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}
