use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn unnatural_selection_firewater_uses_exact_carried_beer_and_completes_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("unnatural-firewater", || {
        super::quest_world_callbacks_fixtures::scenario(
            "unnatural-firewater",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!("POSTGRES_CASE_COMPLETED unnatural-firewater success={}", result.is_ok());
    result
}
