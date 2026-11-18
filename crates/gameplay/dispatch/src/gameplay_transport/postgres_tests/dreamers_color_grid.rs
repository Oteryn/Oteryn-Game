//! Crystal 201189f data-global/scripts/quests/dreamers_challenge_quest/
//! movement_riddle.lua:3-54 and movement_riddle_teleport.lua:11-58.
use super::TestResult;
use super::fixture::debug;
use super::quest_world_callbacks::{local, scenario, wire, world_cells};
use crate::gameplay_transport::{AdmittedSession, ComposedFreshAdmission};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, GridGroup, Position};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

const RIGHT: &str = "dreamers-grid-shift";
const SOLVED: &str = "dreamers-grid-solved";
const UNSOLVED: &str = "dreamers-grid-unsolved";
const COLORS: [u16; 4] = [2394, 2396, 2395, 2397];
const SHIFT: Position = at(32816, 32334, -9);
const TRIGGER: Position = at(32819, 32347, -9);
const DESTINATION: Position = at(32766, 32275, -14);
const PATTERN: [u16; 6] = [2394, 2395, 2396, 2397, 2394, 2395];

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

fn trigger(name: &str) -> Position {
    if name == RIGHT { SHIFT } else { TRIGGER }
}

pub(super) fn matches(name: &str) -> bool {
    matches!(name, RIGHT | SOLVED | UNSOLVED)
}

fn groups() -> Vec<GridGroup> {
    [(0, 0), (3, 0), (0, 3), (3, 3)].into_iter().zip(COLORS).map(|((dx, dy), color)| {
        let cells = (0..3).flat_map(|x| (0..3).map(move |y| at(32818 + dx + x, 32334 + dy + y, -9))).collect();
        GridGroup { cells, colors: if color == 2397 { vec![2397, 900] } else { vec![color] } }
    }).collect()
}

pub(super) fn binding(name: &str) -> TestResult<Callback> {
    let (selector, grid, destination, refund, effect, source) = if name == RIGHT {
        (serde_json::json!({"position":SHIFT,"action":2245,"appearance":431}),
         serde_json::json!({"mode":"shift","axis":"x","offset":2,"length":6,"colors":[2394,2395,2396,2397,900]}),
         None, false, serde_json::json!({"transform":430}), "movement_riddle.lua:3-54")
    } else {
        (serde_json::json!({"position":TRIGGER,"unique":50147,"appearance":1949}),
         serde_json::json!({"mode":"reset","groups":groups()}),
         Some(DESTINATION), true, serde_json::json!({"teleport_effect":11}), "movement_riddle_teleport.lua:11-58")
    };
    let mut effects = effect;
    effects["color_grid"] = grid;
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/dreamers-challenge-color-grid",
        "quest":"oteryn:quest.dreamer_s_challenge_quest",
        "trigger":"step_in","selector":selector,"reversible":false,
        "passages":[],"storage":[],"offerings":[],
        "teleport":destination,"return_on_failure":refund,"effects":effects,
        "sources":[format!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/{source}")]
    }))?)
}

pub(super) fn world(callback: &Callback, name: &str) -> TestResult<crate::map::boot::BundleWorld> {
    let mut objects = if name == RIGHT {
        (0..6).map(|x| at(32818 + x, 32334, -9)).zip(PATTERN).collect::<Vec<_>>()
    } else {
        groups().into_iter().flat_map(|group| {
            let color = if group.colors.len() > 1 { 900 } else { group.colors[0] };
            group.cells.into_iter().map(move |position| (position, color))
        }).collect()
    };
    if name == UNSOLVED {
        objects[0].1 = 2395;
    }
    world_cells(
        callback, trigger(name), callback.selector.appearance.ok_or("source appearance")?, 1,
        &objects, &[430, 2394, 2395, 2396, 2397], None, &[],
    )
}

#[test]
fn dreamers_color_grid_step_rotates_the_six_cells_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(RIGHT, || scenario(RIGHT, SHIFT, false))
}

#[test]
fn dreamers_color_grid_solved_quadrants_reset_to_nine_of_each_color() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(SOLVED, || scenario(SOLVED, TRIGGER, false))
}

#[test]
fn dreamers_color_grid_mixed_quadrant_returns_the_player_without_reset() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(UNSOLVED, || scenario(UNSOLVED, TRIGGER, false))
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    name: &str,
) -> TestResult {
    use crate::map::view::MapFacts;
    let map = owner.bundle_map().ok_or("grid map")?;
    let read = |position: Position| -> TestResult<u16> {
        let pos = crate::map::overlay::TilePos { x: position.x, y: position.y, floor: position.floor };
        let base = map.overlay.base().tile(pos.x, pos.y, pos.floor).ok_or("color cell")?;
        let facts = map.facts.base_entry(pos, 1, *base.ids().get(1).ok_or("color item")?)
            .ok_or("current color")?;
        Ok(facts.appearance_id)
    };
    let cells = if name == RIGHT {
        (0..6).map(|x| at(32818 + x,32334,-9)).collect::<Vec<_>>()
    } else {
        groups().into_iter().flat_map(|group| group.cells).collect()
    };
    let before = cells.iter().copied().map(read).collect::<Result<Vec<_>, _>>()?;
    let reply = wire(owner, admitted, 1, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, encode_step_intent(StepDirection::East)).await?;
    assert_eq!(decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let landing = match name {
        RIGHT => SHIFT,
        SOLVED => DESTINATION,
        _ => at(TRIGGER.x - 1, TRIGGER.y, TRIGGER.floor),
    };
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), local(landing));
    let after = cells.iter().copied().map(read).collect::<Result<Vec<_>, _>>()?;
    match name {
        RIGHT => {
            assert_eq!(after, vec![2395,2394,2395,2396,2397,2394]);
            assert_eq!(read(SHIFT)?, 430);
        }
        SOLVED => {
            for color in COLORS {
                assert_eq!(after.iter().filter(|other| **other == color).count(), 9);
            }
        }
        _ => assert_eq!(before, after),
    }
    Ok(())
}
