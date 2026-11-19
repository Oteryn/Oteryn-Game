use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, annihilator_route_fixtures::{step,use_target},
    fixture::debug, l6_shared_gate_native_fixtures::relocate, quest_demonrage_fixtures::{appearance,target},
    quest_world_callbacks_fixtures::local,
};
use crate::map::boot::BundleMap;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::{self,Callback,Position},
};
use oteryn_protocol_oteryn::{world_object::UseDisposition, world_spatial::StepDirection};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";
const SEAL: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/sixth_seal";
const DOOR: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/sixth_seal_door";
const LEVER: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/action-6-sixth_seal_lever.lua:3-32";
const FLAME: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-6-sixth_seal_flame.lua:3-35";
const DIAGRAM: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/quests/the_queen_of_the_banshees.lua:108-125";
const COLUMNS: [u16; 3] = [32310,32312,32314];
const ROWS: [u16; 2] = [31975,31976];
pub(super) const START: Position = at(32311,31977,-13);
const LANDING: Position = at(32261,31856,-15);

const fn at(x: u16,y: u16,floor: i8) -> Position {
    Position {x,y,floor}
}


pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    map: &BundleMap,
) -> TestResult {
    let mut callbacks = Vec::new();
    let mut requirements = Vec::new();
    for x in COLUMNS {
        for y in ROWS {
            let fire = at(x-1,y,-13);
            requirements.push(json!({"position":fire,"appearance":if x == 32314 {1998} else {1996}}));
            callbacks.push(serde_json::from_value::<Callback>(json!({
                "key":format!("oteryn:quest-callback/logic-switch-{x}-{y}"),"quest":QUEST,
                "trigger":"use","selector":{"position":at(x,y,-13),"ordinal":1,"appearance":2772},
                "lever":[2772,2773],"reversible":true,"return_on_failure":false,
                "passages":[],"offerings":[],"storage":[],
                "effects":{"scene":{
                    "open":[{"position":fire,"remove":1998,"create":1996,"passable":true,"allow_missing":true}],
                    "close":[{"position":fire,"remove":1996,"create":1998,"passable":true,"allow_missing":true}]
                }},
                "sources":[LEVER,DIAGRAM]
            }))?);
        }
    }
    let flame = json!({
        "key":"oteryn:quest-callback/logic-sixth-seal","quest":QUEST,"trigger":"step_in",
        "selector":{"native":{"position":at(32311,31978,-13),"ordinal":1,"appearance":1959}},
        "requires_world_admission":true,"return_on_failure":true,"reversible":false,
        "passages":[],"offerings":[],"world_requirements":requirements,"teleport":LANDING,
        "storage":[{"track":SEAL,"minimum":0,"maximum":0,"set":1},
                   {"track":DOOR,"minimum":-1,"maximum":1,"set":1}],
        "effects":{"teleport_effect":11},
        "sources":[FLAME,DIAGRAM]
    });
    let recipe: String = Sha256::digest(serde_json::to_vec(&flame)?)
        .iter().map(|byte| format!("{byte:02x}")).collect();
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,"quests":[{
            "quest":QUEST,"recipe_sha256":recipe,"sources":[LEVER,FLAME,DIAGRAM],
            "tracks":[{"key":SEAL,"initial":0,"minimum":0,"maximum":1},
                      {"key":DOOR,"initial":-1,"minimum":-1,"maximum":1}],
            "stages":[{"key":"flame","completes":false,"trigger":{"kind":"step_in","callback":flame}}]
        }]
    }))?)?;
    let mut callbacks = world_callbacks::load_with_source_fixture(&callbacks)?;
    callbacks.extend(bindings.world_callbacks().cloned());
    assert!(owner.states.quest_callbacks.fixture.set(callbacks).is_ok());
    let base = QuestStateCatalogue::empty("content-1").ok_or("Logic catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let actor = member.runtime_actor.ok_or("Logic operator")?;
    let session = member.game_session_id;
    // Turn on both right-hand campfires as well as the four required lit ones.
    for x in COLUMNS {
        for y in ROWS {
            let standing_y = if y == 31975 {31974} else {31977};
            relocate(owner,actor,session,(x,standing_y,13)).await?;
            assert_eq!(use_target(owner,member,&target(map,at(x,y,-13))?).await?,UseDisposition::Committed);
            assert_eq!(appearance(map,at(x-1,y,-13))?,1996);
        }
    }
    relocate(owner,actor,session,(START.x,START.y,13)).await?;
    step(owner,member,StepDirection::South).await?;
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),local(START));
    for y in ROWS {
        let standing_y = if y == 31975 {31974} else {31977};
        relocate(owner,actor,session,(32314,standing_y,13)).await?;
        assert_eq!(use_target(owner,member,&target(map,at(32314,y,-13))?).await?,UseDisposition::Committed);
        assert_eq!(appearance(map,at(32313,y,-13))?,1998);
    }
    relocate(owner,actor,session,(START.x,START.y,13)).await?;
    step(owner,member,StepDirection::South).await?;
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),local(LANDING));
    let fence = owner.current_quest_fence(session).await.map_err(|_| "Logic fence")?.ok_or("Logic session")?;
    let state = owner.root.read_character_quest_state(owner.character,fence.character_id).await.map_err(debug)?;
    assert_eq!(state.tracks().get(SEAL),Some(&1));
    assert_eq!(state.tracks().get(DOOR),Some(&1));
    Ok(())
}
