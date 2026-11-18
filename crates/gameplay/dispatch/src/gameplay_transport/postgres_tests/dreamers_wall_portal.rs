//! Crystal 201189f data-global/scripts/quests/dreamers_challenge_quest/movement_wall_teleport.lua:1-43.
use super::TestResult;
use super::quest_world_callbacks::{stateless_scenario, world_cells};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

pub(super) const BLOCKED: &str = "dreamers-wall-portal-blocked";
pub(super) const CLEAR: &str = "dreamers-wall-portal-clear";
// Crystal data-global/startup/tables/item.lua:4495-4498 maps UID 9030 here.
const TRIGGER: Position = at(32762, 32290);
const PUZZLE: Position = at(32762, 32305);
const EXIT: Position = at(32852, 32287);
const WALLS: [Position; 16] = [
    at(32760, 32288),
    at(32761, 32288),
    at(32762, 32288),
    at(32763, 32288),
    at(32764, 32288),
    at(32764, 32289),
    at(32764, 32290),
    at(32764, 32291),
    at(32764, 32292),
    at(32763, 32292),
    at(32762, 32292),
    at(32761, 32292),
    at(32760, 32292),
    at(32760, 32291),
    at(32760, 32290),
    at(32760, 32289),
];

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}

pub(super) fn binding() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/dreamers-challenge-wall-portal",
        "quest": "oteryn:quest.dreamer_s_challenge_quest",
        "trigger": "step_in",
        "selector": {"native":{"position":TRIGGER,"ordinal":1,"appearance":1949}},
        "reversible":false, "passages":[], "storage":[], "offerings":[],
        "teleport": EXIT,
        "effects": {
            "solid_teleport": {"cells": WALLS, "destination": PUZZLE},
            "teleport_effect": 11
        },
        "return_on_failure":false,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/movement_wall_teleport.lua:1-43"]
    }))?)
}

pub(super) fn world(
    callback: &Callback,
    blocked: bool,
) -> TestResult<crate::map::boot::BundleWorld> {
    let mut cells = WALLS.to_vec();
    cells.push(PUZZLE);
    // The first item is movable solid furniture. Only the second (if present)
    // carries the donor immovable-solid flag.
    let mut objects = vec![(WALLS[0], 2025)];
    if blocked {
        objects.push((WALLS[15], 100));
    }
    world_cells(callback, TRIGGER, 1949, 1, &objects, &[], None, &cells)
}

#[test]
fn dreamers_wall_portal_redirects_when_any_live_cell_has_an_immovable_solid() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(BLOCKED, || {
        stateless_scenario(BLOCKED, TRIGGER, PUZZLE)
    })
}

#[test]
fn dreamers_wall_portal_passes_when_only_movable_solids_remain() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CLEAR, || stateless_scenario(CLEAR, TRIGGER, EXIT))
}
