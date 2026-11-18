//! Crystal 201189f8 action-3-third_seal_lever.lua:3-45 and movement-3-third_seal_flame.lua:3-31.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::map::{boot::BundleMap, overlay::TilePos, view::{MapFacts, placement_key}};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position,
};
use oteryn_protocol_oteryn::{
    world_object::{UseDisposition, WorldObjectTarget}, world_spatial::StepDirection,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
const SEAL: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/third_seal";
const DOOR: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/third_seal_door";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/action-3-third_seal_lever.lua:3-45";
const EFFECTS: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/quests/the_queen_of_the_banshees.lua:26-88";
const FLAME: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-3-third_seal_flame.lua:3-31";
const ORDER: [u16; 5] = [31843, 31845, 31844, 31842, 31846];
const DIAGRAM: [&[(u16,u16)]; 5] = [
    &[(32217,31845),(32218,31845),(32219,31845),(32220,31845),(32217,31843),(32218,31842),(32219,31841)],
    &[(32217,31844),(32218,31844),(32219,31843),(32220,31845),(32219,31845)],
    &[(32217,31842),(32219,31843),(32219,31845),(32218,31844),(32217,31844),(32217,31845)],
    &[(32217,31845),(32218,31846),(32218,31844),(32219,31845),(32220,31846)],
    &[(32219,31841),(32219,31842),(32219,31846),(32217,31843),(32217,31844),(32217,31845),(32218,31843),(32218,31845)],
];

pub(super) const START: Position = at(32218,31844,-15);

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position {x,y,floor}
}

fn tile(p: Position) -> TilePos {
    TilePos {x:p.x,y:p.y,floor:p.floor}
}

fn stages() -> Vec<Value> {
    let basin_positions = [32214,32215,32216].map(|x| at(x,31850,-15));
    let mut stages = Vec::new();
    for (index, &y) in ORDER.iter().enumerate() {
        let position = at(32220,y,-15);
        let requirements: Vec<_> = ORDER[..index].iter().map(|&prior| {
            json!({"position":at(32220,prior,-15), "appearance":2772})
        }).collect();
        let advance: Vec<_> = if index + 1 == ORDER.len() {
            basin_positions.iter().map(|pos| json!({"position":pos,"appearance":2114,"replacement":2113})).collect()
        } else {
            Vec::new()
        };
        let success: Vec<_> = DIAGRAM[index].iter().map(|&(x,y)| json!({"position":at(x,y,-14),"effect":12})).collect();
        let mut cues = json!({"success":success});
        if index != 0 {
            cues["wrong_order"] = json!({"position":position,"effect":12});
        }
        stages.push(json!({
            "key":format!("switch_{}",index+1),"completes":false,
            "trigger":{"kind":"map_use","callback":{
                "key":format!("oteryn:quest-callback/demonrage-switch-{}",index+1),"quest":QUEST,
                "trigger":"use","selector":{"position":position,"ordinal":1,"appearance":2773},
                "lever":[2773,2772],"reversible":false,"passages":advance,"offerings":[],
                "world_requirements":requirements,"return_on_failure":false,
                "storage":[{"track":SEAL,"minimum":0,"maximum":0,"set":0,"read_only":true}],
                "effects":{"magic":{"position":position,"effect":13},"switch_cues":cues},
                "sources":[SOURCE,EFFECTS]
            }}
        }));
    }
    let requirements: Vec<_> = ORDER.iter().map(|&y| {
        json!({"position":at(32220,y,-15),"appearance":2772})
    }).collect();
    let reset: Vec<_> = ORDER.iter().map(|&y| {
        json!({"position":at(32220,y,-15),"appearance":2772,"replacement":2773})
    }).chain(basin_positions.iter().map(|position| {
        json!({"position":position,"appearance":2113,"replacement":2114})
    })).collect();
    stages.push(json!({
        "key":"flame","completes":false,
        "trigger":{"kind":"step_in","callback":{
            "key":"oteryn:quest-callback/demonrage-third-seal","quest":QUEST,"trigger":"step_in",
            "selector":{"native":{"position":at(32215,31849,-15),"ordinal":1,"appearance":1959}},
            "requires_world_admission":true,"return_on_failure":true,
            "passages":reset,"offerings":[],"reversible":false,
            "world_requirements":requirements,"teleport":at(32271,31857,-15),
            "storage":[{"track":SEAL,"minimum":0,"maximum":0,"set":1},
                       {"track":DOOR,"minimum":-1,"maximum":1,"set":1}],
            "effects":{"teleport_effect":11},"sources":[FLAME,EFFECTS]
        }}
    }));
    stages
}

fn target(map: &BundleMap, p: Position) -> TestResult<WorldObjectTarget> {
    let key = placement_key(tile(p), 1).ok_or("Demonrage lever placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {placement,expected_revision:map.facts.object_revision(key)})
}

fn appearance(map: &BundleMap, p: Position) -> TestResult<u16> {
    let t = map.overlay.base().tile(p.x,p.y,p.floor).ok_or("Demonrage tile")?;
    let id = t.ids().iter().zip(t.depths()).filter(|(_,d)| **d == 0).nth(1)
        .map(|(&id,_)| id).ok_or("Demonrage slot")?;
    Ok(map.facts.base_entry(tile(p),1,id).ok_or("Demonrage object")?.appearance_id)
}

#[test]
fn demonrage_uses_source_cues_and_resets_shared_switches_for_the_flame_entrant() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::demonrage_switches(admin)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [AdmittedSession],
    map: &BundleMap,
) -> TestResult {
    use super::{
        annihilator_route::{step,use_target},
        l6_shared_gate_native::relocate,
        quest_world_callbacks::local,
    };

    let stages = stages();
    let recipe: String = Sha256::digest(serde_json::to_vec(&stages)?)
        .iter().map(|byte| format!("{byte:02x}")).collect();
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,"quests":[{
            "quest":QUEST,"recipe_sha256":recipe,"sources":[SOURCE,EFFECTS,FLAME],
            "tracks":[{"key":SEAL,"initial":0,"minimum":0,"maximum":1},
                      {"key":DOOR,"initial":-1,"minimum":-1,"maximum":1}],
            "stages":stages
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("Demonrage catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let [operator, entrant] = members else { return Err("expected two Demonrage characters".into()); };
    let operator_actor = operator.runtime_actor.ok_or("switch operator")?;
    let entrant_actor = entrant.runtime_actor.ok_or("flame entrant")?;

    let second = at(32220,ORDER[1],-15);
    relocate(owner,operator_actor,operator.game_session_id,(32219,second.y,15)).await?;
    assert_eq!(use_target(owner,operator,&target(map,second)?).await?,UseDisposition::Rejected);
    assert_eq!(appearance(map,second)?,2773);
    let cues = map.facts.movement_effects_since(0);
    assert_eq!(cues.iter().map(|(_,p,e)| (*p,*e)).collect::<Vec<_>>(),[(tile(second),12)]);
    let mut since = cues[0].0;
    for (index,&y) in ORDER.iter().enumerate() {
        let lever = at(32220,y,-15);
        let standing_y = if y == 31843 {31844} else {y};
        relocate(owner,operator_actor,operator.game_session_id,(32219,standing_y,15)).await?;
        assert_eq!(use_target(owner,operator,&target(map,lever)?).await?,UseDisposition::Committed);
        assert_eq!(appearance(map,lever)?,2772);
        let effects = map.facts.movement_effects_since(since);
        let expected: Vec<_> = std::iter::once((tile(lever),13))
            .chain(DIAGRAM[index].iter().map(|&(x,y)| (tile(at(x,y,-14)),12))).collect();
        assert_eq!(effects.iter().map(|(_,p,e)| (*p,*e)).collect::<Vec<_>>(),expected);
        since = effects.last().ok_or("committed switch cues")?.0;
    }
    for x in 32214..=32216 {
        assert_eq!(appearance(map,at(x,31850,-15))?,2113);
    }
    relocate(owner,entrant_actor,entrant.game_session_id,(32215,31848,15)).await?;
    step(owner,entrant,StepDirection::South).await?;
    assert_eq!(owner.runtime.lock().await.read_actor_position(entrant_actor).map_err(debug)?.position(),local(at(32271,31857,-15)));
    for y in ORDER {
        assert_eq!(appearance(map,at(32220,y,-15))?,2773);
    }
    for x in 32214..=32216 {
        assert_eq!(appearance(map,at(x,31850,-15))?,2114);
    }
    for (member, expected) in [(&*operator,None),(&*entrant,Some(&1))] {
        let fence = owner.current_quest_fence(member.game_session_id).await.map_err(|_| "Demonrage fence")?.ok_or("session")?;
        let state = owner.root.read_character_quest_state(owner.character,fence.character_id).await.map_err(debug)?;
        assert_eq!(state.tracks().get(SEAL),expected);
        assert_eq!(state.tracks().get(DOOR),expected);
    }
    Ok(())
}
