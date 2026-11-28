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

#[test]
fn token_exchanges_retains_unfinished_save_pins() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    for scenario in ["token-exchanges#published-save", "token-exchanges#train-save"] {
        super::kill_reward_e2e_fixtures::scenario_thread::run(scenario, move || {
            super::quest_world_callbacks_fixtures::scenario(scenario, Position { x: 33294, y: 31677, floor: -15 }, false)
        })?;
    }
    Ok(())
}

#[test]
fn regional_hunt_save_retains_unfinished_save_pins() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    for scenario in ["regional-hunt-save#published-save", "regional-hunt-save#train-save"] {
        super::kill_reward_e2e_fixtures::scenario_thread::run(scenario, move || {
            super::quest_world_callbacks_fixtures::scenario(scenario, Position { x: 33294, y: 31677, floor: -15 }, false)
        })?;
    }
    Ok(())
}

#[test]
fn authored_wrath_opening_retains_unfinished_save_pins() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    for scenario in ["authored-wrath-opening#published-save", "authored-wrath-opening#train-save"] {
        super::kill_reward_e2e_fixtures::scenario_thread::run(scenario, move || {
            super::quest_world_callbacks_fixtures::scenario(scenario, Position { x: 33354, y: 31410, floor: -8 }, false)
        })?;
    }
    Ok(())
}
