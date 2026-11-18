//! Crystal 201189f8, ferumbras_ascension/actions_flower_puzzle_lever.lua:1-11,29-37,63-66.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::json;

pub(super) const CYCLE: &str = "ferumbras-shulgrax-flower-step-cycle";
pub(super) const CASE: &str = "ferumbras-shulgrax-flower-pattern";
// The fixture places AID 34300 beside the source gates. Lua selects by AID.
pub(super) const SWITCH: Position = at(33452, 32727);
const SWITCH_FLOWERS: [Position; 3] = [at(33455, 32707), at(33455, 32708), at(33455, 32709)];
const LEVER: Position = at(33477, 32696);
const FLOWERS: [(Position, u16); 9] = [
    (at(33455, 32708), 3676),
    (at(33459, 32707), 3676),
    (at(33460, 32708), 3676),
    (at(33456, 32692), 3678),
    (at(33458, 32692), 3678),
    (at(33460, 32693), 3678),
    (at(33457, 32686), 3677),
    (at(33459, 32686), 3677),
    (at(33456, 32687), 3677),
];

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}

#[test]
fn shulgrax_lever_requires_all_nine_generated_flowers_in_their_original_positions() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CASE, || {
        super::quest_world_callbacks::scenario(CASE, LEVER, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    let requirements =
        FLOWERS.map(|(position, appearance)| json!({"position":position,"appearance":appearance}));
    let data = json!({"schema":1,"callbacks":[{
        "key":"oteryn:quest-callback/ferumbras/shulgrax/flower-pattern",
        "quest":"oteryn:quest.ferumbras_ascension_quest",
        "trigger":"use","selector":{"position":LEVER,"action":34300,"appearance":9110},
        "lever":[9110,9111],"reversible":true,"passages":[],"storage":[],"offerings":[],
        "return_on_failure":false,
        "effects":{"generated_item_requirements":requirements},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/actions_flower_puzzle_lever.lua:1-11,29-37,63-66"]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&data.to_string())?
        .try_into()
        .map_err(|_| "one Shulgrax lever callback")?;
    Ok(callback)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_scene_cells(
        callback,
        LEVER,
        &[],
        &[3676, 3677, 3678, 9111],
        &FLOWERS.map(|(position, _)| position),
    )
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    callback: Callback,
) -> TestResult {
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback])
        .map_err(|_| "callback installed")?;
    let map = owner.bundle_map().ok_or("map")?;
    let create = |position: Position, appearance| {
        let cell = crate::map::overlay::TilePos {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        assert!(map.facts.create_movement_item(cell, appearance, 0, || true));
    };
    let [(last, expected), first @ ..] = FLOWERS;
    for (position, appearance) in first {
        create(position, appearance);
    }
    create(last, 3677);
    assert_eq!(
        super::questpc7_ferumbras::use_at(owner, admitted, LEVER, 9110).await?,
        UseDisposition::Rejected
    );
    create(last, expected);
    assert_eq!(
        super::questpc7_ferumbras::use_at(owner, admitted, LEVER, 9110).await?,
        UseDisposition::Committed
    );
    Ok(())
}

// Crystal 201189f8, ferumbras_ascension/movements_flower_puzzle.lua:74-79,125-161.
#[test]
fn shulgrax_floor_switch_walk_cycles_through_three_flower_positions_and_clears() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CYCLE, || {
        super::quest_world_callbacks::scenario(CYCLE, SWITCH, false)
    })
}

pub(super) fn cycle_callback() -> TestResult<Callback> {
    let data = json!({"schema":1,"callbacks":[{
        "key":"oteryn:quest-callback/ferumbras/shulgrax/flower-cycle/13",
        "quest":"oteryn:quest.ferumbras_ascension_quest",
        "trigger":"step_in","selector":{"position":SWITCH,"appearance":22538},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"item_layout_cycle":{
            "source_appearances":[22538,22539,22540,22541],
            "positions":SWITCH_FLOWERS,"appearance":3676,"clear_appearances":[3676,3677,3678]
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/movements_flower_puzzle.lua:74-79,125-161"]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&data.to_string())?
        .try_into()
        .map_err(|_| "one Shulgrax switch callback")?;
    Ok(callback)
}

pub(super) fn cycle_world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_scene_cells(
        callback,
        SWITCH,
        &[],
        &[22539, 22540, 22541, 3676],
        &SWITCH_FLOWERS,
    )
}

pub(super) async fn exercise_cycle(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    callback: Callback,
) -> TestResult {
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_spatial::StepDirection;

    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback])
        .map_err(|_| "callback installed")?;
    let map = owner.bundle_map().ok_or("map")?;
    let tile = |p: Position| crate::map::overlay::TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    };
    let source = crate::map::view::placement_key(tile(SWITCH), 1).ok_or("switch placement")?;
    for (index, expected) in [Some(0), Some(1), Some(2), None].into_iter().enumerate() {
        super::questpc7_ferumbras::step(owner, admitted, StepDirection::East).await?;
        assert_eq!(map.facts.object_revision(source), (index + 1) as u64);
        for (slot, position) in SWITCH_FLOWERS.into_iter().enumerate() {
            let entries = map.facts.movement_items(tile(position));
            assert_eq!(
                entries
                    .iter()
                    .map(|entry| entry.facts.appearance_id)
                    .collect::<Vec<_>>(),
                if expected == Some(slot) {
                    vec![3676]
                } else {
                    Vec::new()
                }
            );
        }
        super::questpc7_ferumbras::step(owner, admitted, StepDirection::West).await?;
    }
    Ok(())
}
