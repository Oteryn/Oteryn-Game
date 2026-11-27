use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn token_exchanges_and_vampire_ranks_over_npc_wire() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("token-exchanges", || {
        super::quest_world_callbacks_fixtures::scenario(
            "token-exchanges",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}
