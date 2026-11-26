//! Interrupted quest payments through the shutdown and control-loss owners.
use super::{TestResult, desert_party_rite_fixtures, quest_world_callbacks_fixtures};
use oteryn_gameplay_quests::quest::{carried_step, world_callbacks::Position};

#[test]
fn desert_dungeon_shutdown_drains_paid_rite_before_position_save() -> TestResult {
    run(|| desert_party_rite_fixtures::scenario_with_shutdown(false, true))
}

#[test]
fn l6_tomb_carried_debit_settles_before_control_loss() -> TestResult {
    run(|| {
        let recipe = carried_step::RECIPES
            .iter()
            .find(|recipe| recipe.unique == 3950)
            .ok_or("source Tomb forcefield recipe")?;
        quest_world_callbacks_fixtures::stateless_scenario(
            "l6-tomb-carried-present-3950/control-loss",
            Position {
                x: 7,
                y: 1,
                floor: -7,
            },
            recipe.success,
        )
    })
}

#[test]
fn dread_lord_paid_scroll_survives_rebuilt_map() -> TestResult {
    run(|| {
        quest_world_callbacks_fixtures::scenario(
            "dreamer-dread-lord-scroll/rebuild",
            super::m07_dreamer_levers_fixtures::DREAD_LORD_LEVER,
            false,
        )
    })
}

fn run(scenario: impl FnOnce() -> TestResult + Send + 'static) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "paid callback scenario panicked")??;
    Ok(())
}
