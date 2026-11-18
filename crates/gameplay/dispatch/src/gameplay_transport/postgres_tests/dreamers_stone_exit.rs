//! Crystal 201189f dreamers_challenge_quest/movement_stone_teleport.lua:1-33.
use super::fixture::debug;
use super::quest_world_callbacks::{local, scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

pub(super) const NAME: &str = "dreamers-stone-exit";
const PORTAL: Position = at(32881, 32270);
const EXIT: Position = Position {x:32920,y:32296,floor:-13};
const OBJECTS: [(Position, u16, u16); 5] = [
    (at(32873,32263), 2773, 2772),
    (at(32874,32263), 2098, 2094),
    (at(32875,32263), 2099, 2095),
    (at(32874,32264), 2100, 2096),
    (at(32875,32264), 2101, 2097),
];

const fn at(x: u16, y: u16) -> Position {
    Position { x,y,floor:-14 }
}

fn tile(p: Position) -> TilePos {
    TilePos { x:p.x,y:p.y,floor:p.floor }
}

pub(super) fn binding() -> TestResult<Callback> {
    let passages: Vec<_> = OBJECTS.iter().map(|&(position, appearance, replacement)| {
        serde_json::json!({"position":position, "appearance":appearance,
            "replacement":replacement,"if_present":true})
    }).collect();
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/dreamers-challenge-stone-exit",
        "quest":"oteryn:quest.dreamer_s_challenge_quest", "trigger":"step_in",
        "selector":{"position":PORTAL, "appearance":1949},
        "teleport":EXIT,"return_on_failure":false,"reversible":false,
        "storage":[],"offerings":[],"passages":passages,
        "effects":{"portal_reset_passable":false,"transform":1842,"teleport_effect":11},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/movement_stone_teleport.lua:1-33"]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    let mut present = callback.clone();
    // One well was already restored when the player entered.
    present.passages.pop();
    world_cells(&present, PORTAL, 1949, 1, &[(OBJECTS[4].0,OBJECTS[4].2)], &[1842], None, &[])
}

#[test]
fn dreamers_stone_portal_step_restores_stone_and_only_active_wells() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, PORTAL, false))
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let reply = wire(owner, admitted, 1, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, encode_step_intent(StepDirection::East)).await?;
    assert_eq!(decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), local(EXIT));
    let map = owner.bundle_map().ok_or("map")?;
    let base = map.overlay.base();
    for (position, appearance, revision) in OBJECTS.iter().enumerate()
        .map(|(index, &(position, _, appearance))| (position, appearance, u64::from(index < 4)))
        .chain([(PORTAL, 1842, 1)])
    {
        let pos = tile(position);
        let id = base.tile(pos.x, pos.y, pos.floor).ok_or("tile")?.ids()[1];
        let facts = map.facts.base_entry(pos, 1, id).ok_or("object")?;
        assert_eq!(facts.appearance_id, appearance);
        let key = crate::map::view::placement_key(pos, 1).ok_or("entry")?;
        assert_eq!(map.facts.object_revision(key), revision);
    }
    assert!(map.facts.quest_callback_blocked(base, tile(PORTAL), false));
    Ok(())
}
