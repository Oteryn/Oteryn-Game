//! Crystal 201189f data-global/scripts/quests/dreamers_challenge_quest/movement_wall_teleport.lua:1-43.
use super::TestResult;
use super::quest_world_callbacks::{stateless_scenario, world_cells};
use crate::map::{overlay::TilePos, view::placement_key};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

pub(super) const BLOCKED: &str = "dreamers-wall-portal-blocked";
pub(super) const CLEAR: &str = "dreamers-wall-portal-clear";
pub(super) const FOREIGN_UID: &str = "dreamers-wall-portal-foreign-uid";
pub(super) const LIVE_CLEAR: &str = "dreamers-wall-portal-live-clear";
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
        "selector": {"unique":9030,"position":TRIGGER,"appearance":1949,
            "native":{"position":TRIGGER,"ordinal":1,"appearance":1949}},
        "reversible":false, "passages":[], "storage":[], "offerings":[],
        "teleport": EXIT,
        "effects": {
            "solid_teleport": {"cells": WALLS, "destination": PUZZLE},
            "teleport_effect": 11
        },
        "return_on_failure":false,
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/movement_wall_teleport.lua:1-43",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:4495-4498"
        ]
    }))?)
}

pub(super) fn world(
    callback: &Callback,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    let mut cells = WALLS.to_vec();
    cells.push(PUZZLE);
    // The first item is movable solid furniture. Only the second (if present)
    // carries the donor immovable-solid flag.
    let mut objects = vec![(WALLS[0], 2025)];
    if matches!(name, BLOCKED | LIVE_CLEAR) {
        objects.push((WALLS[15], 100));
    }
    // Keep the native counterpart witness on the map that strips the startup UID.
    let mut map_fixture = callback.clone();
    map_fixture.selector.unique = match name {
        CLEAR => None,
        FOREIGN_UID => Some(9033), // Crystal startup item.lua:4499-4503 assigns it elsewhere.
        _ => callback.selector.unique,
    };
    world_cells(&map_fixture, TRIGGER, 1949, 1, &objects, &[], None, &cells)
}

pub(super) fn clear_live_wall(map: &crate::map::boot::BundleMap) -> TestResult {
    let wall = WALLS[15];
    let position = TilePos { x: wall.x, y: wall.y, floor: wall.floor };
    let key = placement_key(position, 1).ok_or("wall placement")?;
    let open = map.facts.quest_callback_replacement(key, 2025).ok_or("movable solid replacement")?;
    assert_eq!(map.facts.immovable_solid_at(&map.overlay, position), Some(true));
    assert!(map.facts.apply_quest_callback(key, 0, open, &[], &[], false, None));
    assert_eq!(map.facts.immovable_solid_at(&map.overlay, position), Some(false));
    Ok(())
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

#[test]
fn dreamers_wall_portal_reads_the_current_wall_after_a_live_transform() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(LIVE_CLEAR, || {
        stateless_scenario(LIVE_CLEAR, TRIGGER, EXIT)
    })
}

#[test]
fn dreamers_wall_portal_does_not_take_a_native_counterpart_with_a_foreign_uid() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(FOREIGN_UID, || {
        stateless_scenario(FOREIGN_UID, TRIGGER, TRIGGER)
    })
}
