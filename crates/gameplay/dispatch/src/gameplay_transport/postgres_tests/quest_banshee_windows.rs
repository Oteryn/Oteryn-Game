//! Crystal 201189f8 action-1-first_seal_lever.lua:3-45 and movement-1-first_seal_close_mw.lua:1-73.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    annihilator_route::{step, use_target},
    fixture::debug,
    l6_shared_gate_native::relocate,
    quest_demonrage::target,
    quest_world_callbacks::local,
};
use crate::map::{boot::BundleMap, overlay::TilePos, view::{MapFacts,placement_key}};
use oteryn_gameplay_quests::quest::world_callbacks::{self,Callback,Position};
use oteryn_protocol_oteryn::{world_object::UseDisposition,world_spatial::StepDirection};
use serde_json::json;
use std::sync::atomic::Ordering;

const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
const LEVER: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/action-1-first_seal_lever.lua:3-45";
const CLOSE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-1-first_seal_close_mw.lua:1-73";
const TABLE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/quests/the_queen_of_the_banshees.lua:2-15";
pub(super) const START: Position = at(32314,31910,-12);
const FIRST: Position = at(32315,31910,-12);
const SECOND: Position = at(32212,31888,-12);
const FIRST_WALL: Position = at(32259,31891,-10);
const SECOND_WALL: Position = at(32259,31890,-10);
pub(super) const TRAPDOOR_LEVER: Position = at(32266,31861,-11);
const TRAPDOOR: Position = at(32266,31860,-11);

const fn at(x: u16,y: u16,floor: i8) -> Position {
    Position {x,y,floor}
}

fn source_bindings() -> TestResult<Vec<Callback>> {
    let mut callbacks = Vec::new();
    for (uid, lever, wall, open) in [
        (30010,FIRST,FIRST_WALL,None),
        (30011,SECOND,SECOND_WALL,None),
        (30012,TRAPDOOR_LEVER,TRAPDOOR,Some(369)),
    ] {
        let effects = if open.is_some() { json!({}) } else { json!({"passages_reset_after_ms":100_000}) };
        let mut passage = json!({"position":wall,"appearance":2129});
        if let Some(open) = open {
            passage["replacement"] = open.into();
        }
        callbacks.push(serde_json::from_value(json!({
            "key":format!("oteryn:quest-callback/banshees-first-seal-switch-{uid}"),"quest":QUEST,
            "trigger":"use","selector":{"unique":uid,"native":{"position":lever,"ordinal":1,"appearance":2772}},
            "lever":[2772,2773],"reversible":true,"return_on_failure":false,
            "passages":[passage],"offerings":[],"storage":[],"effects":effects,
            "sources":[LEVER,TABLE]
        }))?);
    }
    for (step, ground, name, passages, delay) in [
        (at(32315,31913,-13),386,"west",json!([{"position":SECOND_WALL,"appearance":2129}]),120_000),
        (at(32210,31898,-12),410,"east",json!([{"position":FIRST_WALL,"appearance":2129}]),120_000),
        (at(32266,31860,-12),431,"trapdoor",json!([
            {"position":TRAPDOOR,"appearance":2129,"replacement":369},
            {"position":TRAPDOOR_LEVER,"appearance":27260,"replacement":2773},
        ]),3_000),
    ] {
        callbacks.push(serde_json::from_value(json!({
            "key":format!("oteryn:quest-callback/banshees-first-seal-{name}-close"),"quest":QUEST,
            "trigger":"step_in","selector":{"native":{"position":step,"ordinal":0,"appearance":ground}},
            "passages":passages,"lever":null,"reversible":false,"offerings":[],"storage":[],
            "return_on_failure":false,"effects":{"close_passages_after_ms":delay},"sources":[CLOSE]
        }))?);
    }
    Ok(callbacks)
}

#[test]
fn banshee_first_seal_honors_independent_windows_and_the_trapdoor_floor_route() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::banshee_windows(admin)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [AdmittedSession],
    map: &BundleMap,
) -> TestResult {
    let callbacks = world_callbacks::load_with_source_fixture(&source_bindings()?)?;
    assert!(owner.states.quest_callbacks.fixture.set(callbacks).is_ok());
    let [operator, traveler] = members else { return Err("two Banshee participants".into()); };
    let operator_actor = operator.runtime_actor.ok_or("first switch operator")?;
    let traveler_actor = traveler.runtime_actor.ok_or("second switch operator")?;

    // Crystal action-1-first_seal_lever.lua:9-14 creates real ground. Item
    // 369 carries floorchange=down in Crystal data/items/items.xml:744-747.
    // The fixture positions the operator on the retained lever tile before
    // issuing native USE and STEP, independent of the adjacent debris.
    let upper = map.overlay.base().tile(TRAPDOOR.x, TRAPDOOR.y, TRAPDOOR.floor)
        .ok_or("native Banshee trapdoor tile")?;
    assert!(upper.ground().is_none(), "the closed tile holds only its wall");
    owner.states.test_owner_time.store(10_000_001,Ordering::Relaxed);
    assert_eq!(use_target(owner,operator,&target(map,TRAPDOOR_LEVER)?).await?,UseDisposition::Committed);
    step(owner,operator,StepDirection::North).await?;
    let position = owner.runtime.lock().await
        .read_actor_position(operator_actor).map_err(debug)?.position();
    assert_eq!(position, local(at(32266,31860,-12)));
    map.facts.expire_movement_items(13_000_000);
    relocate(owner,operator_actor,operator.game_session_id,(START.x,START.y,12)).await?;

    let walls = [FIRST_WALL,SECOND_WALL].map(|p| TilePos{x:p.x,y:p.y,floor:p.floor});
    let first = placement_key(walls[0],1).ok_or("first wall placement")?;
    let second = placement_key(walls[1],1).ok_or("second wall placement")?;
    assert!(!map.facts.base_hidden(first));
    assert!(!map.facts.base_hidden(second));

    owner.states.test_owner_time.store(100_000_001,Ordering::Relaxed);
    assert_eq!(use_target(owner,operator,&target(map,FIRST)?).await?,UseDisposition::Committed);
    assert!(map.facts.base_hidden(first));
    assert!(!map.facts.base_hidden(second));
    relocate(owner,traveler_actor,traveler.game_session_id,(32211,31888,12)).await?;
    owner.states.test_owner_time.store(105_000_001,Ordering::Relaxed);
    assert_eq!(use_target(owner,traveler,&target(map,SECOND)?).await?,UseDisposition::Committed);
    assert!(map.facts.base_hidden(second));

    // Crystal movement-1-first_seal_close_mw.lua:29-49 arms a close on the
    // other player's wall after 120 seconds.
    relocate(owner,traveler_actor,traveler.game_session_id,(32209,31898,12)).await?;
    owner.states.test_owner_time.store(120_000_001,Ordering::Relaxed);
    step(owner,traveler,StepDirection::East).await?;
    assert_eq!(map.facts.next_movement_decay_time(),Some(200_000_000));
    map.facts.expire_movement_items(199_999_999);
    assert!(map.facts.base_hidden(first));
    map.facts.expire_movement_items(200_000_000);
    assert!(!map.facts.base_hidden(first));
    assert!(map.facts.base_hidden(second));
    assert_eq!(map.facts.next_movement_decay_time(),Some(205_000_000));
    map.facts.expire_movement_items(205_000_000);
    assert!(!map.facts.base_hidden(second));
    assert_eq!(map.facts.next_movement_decay_time(),Some(240_000_000));

    relocate(owner,operator_actor,operator.game_session_id,(START.x,START.y,12)).await?;
    owner.states.test_owner_time.store(220_000_001,Ordering::Relaxed);
    // The donor leaves the lever toggle visible when only its wall expires.
    assert_eq!(use_target(owner,operator,&target(map,FIRST)?).await?,UseDisposition::Committed);
    assert_eq!(use_target(owner,operator,&target(map,FIRST)?).await?,UseDisposition::Committed);
    assert!(map.facts.base_hidden(first));
    map.facts.expire_movement_items(240_000_000);
    assert!(map.facts.base_hidden(first));
    assert_eq!(map.facts.next_movement_decay_time(),Some(320_000_000));
    map.facts.expire_movement_items(320_000_000);
    assert!(!map.facts.base_hidden(first));
    assert_eq!(map.facts.next_movement_decay_time(),None);

    Ok(())
}
