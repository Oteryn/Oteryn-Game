//! Crystal 201189f8 data-global/scripts/quests/spike_tasks/creaturescripts_lower_spike_kill.lua:1-21.
use super::TestResult;
use super::qespike_kill_area_fixtures::{NAME, OUTSIDE_NAME};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

const EDGE: Position = Position {
    x: 32120,
    y: 32470,
    floor: -13,
};

#[test]
fn spike_drillworm_death_keeps_actual_lethal_cell_through_quest_settlement() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, EDGE, false)
    })
}

#[test]
fn spike_drillworm_death_outside_lower_area_leaves_quest_progress_unchanged() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(OUTSIDE_NAME, || {
        super::quest_world_callbacks_fixtures::scenario(
            OUTSIDE_NAME,
            Position {
                x: EDGE.x - 1,
                ..EDGE
            },
            false,
        )
    })
}
