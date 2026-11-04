use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

// zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
// data-global/scripts/movements/teleport/gnomprona.lua:6,30-33.
#[test]
fn quests3_stateless_primal_step_reaches_source_destination_without_quest_mutation() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("quests3-primal-step", || {
        super::quest_world_callbacks::stateless_scenario(
            "oteryn:quest-callback/primal-ordeal-travel-33558-32754-14",
            Position {
                x: 33558,
                y: 32754,
                floor: -14,
            },
            Position {
                x: 33714,
                y: 32799,
                floor: -14,
            },
        )
    })
}
