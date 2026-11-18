//! Crystal 201189f8 elemental_spheres/actions_lever.lua:1-73 and its shared BossRoom gate.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    annihilator_route::use_target,
    fixture::{debug, id},
    l6_shared_gate_native::relocate,
    quest_demonrage::{appearance, target},
    quest_world_callbacks::local,
};
use crate::map::boot::BundleMap;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::json;

const QUEST: &str = "oteryn:quest.elemental_spheres_quest";
const TRACK: &str = "oteryn:quest-progress/elemental_spheres/quest_line";
const LEVER: Position = at(33269, 31835, -10);
const OCCUPIED: &str = "Wait for the current team to exit.";
const INELIGIBLE: &str = "You need one player of each vocation having completed the Elemental Spheres quest and also carrying the elemental rare item.";

pub(super) const START: Position = at(33270, 31835, -10);
pub(super) const PARTIES: [(Position, Position, &str, &str); 4] = [
    (at(33268,31833,-10), at(33268,31833,-12), "sorcerer", "oteryn:item.tibia.i946"),
    (at(33268,31838,-10), at(33267,31838,-12), "druid", "oteryn:item.tibia.i947"),
    (at(33266,31835,-10), at(33265,31835,-12), "knight", "oteryn:item.tibia.i948"),
    (START, at(33270,31835,-12), "paladin", "oteryn:item.tibia.i942"),
];

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

pub(super) fn binding() -> TestResult<Callback> {
    let stations: Vec<_> = PARTIES.iter().map(|(from,to,vocation,item)| {
        json!({"from":from,"to":to,"vocation":vocation,"carried_item":item})
    }).collect();
    Ok(serde_json::from_value(json!({
        "key": "oteryn:quest-callback/lord-of-elements-entry",
        "quest": QUEST,
        "trigger": "use",
        "selector": {
            "unique": 1010,
            "native": {"position":LEVER,"ordinal":1,"appearance":2772}
        },
        "lever": [2772,2773], "reversible": true,
        "passages": [], "storage": [], "offerings": [], "return_on_failure": false,
        "effects": {"party_transfer": {
            "activation_appearance": 2773, "stations": stations,
            "room_entry": {
                "initiator":START, "room_min":at(33238,31806,-12), "room_max":at(33298,31866,-12),
                "window_lever":at(33273,31840,-12), "track":TRACK, "minimum":1,
                "occupied_speech":{"position":at(33268,31835,-10),"text":OCCUPIED},
                "ineligible_speech":{"position":at(33268,31835,-10),"text":INELIGIBLE}
            }
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/elemental_spheres/actions_lever.lua:1-73",
                   "https://www.tibiawiki.com.br/wiki/Elemental_Spheres_Quest"]
    }))?)
}

#[test]
fn lord_chamber_entry_checks_each_member_and_refuses_during_the_window() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::lord_entry(admin)
}

async fn seed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pool: &sqlx::PgPool,
    member: &AdmittedSession,
    index: usize,
    progress: bool,
    item: bool,
) -> TestResult {
    let fence = owner.current_quest_fence(member.game_session_id).await
        .map_err(|_| "entry fixture fence")?.ok_or("entry fixture session")?;
    let character = fence.character_id.as_bytes().as_slice();
    let world = owner.world_id.as_bytes().as_slice();
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    if progress {
        sqlx::query("INSERT INTO game_character_quest_tracks
           (character_id,track_key,quest_key,value,committed_character_revision)
           SELECT character_id,$2,$3,1,character_revision FROM game_character_roots
           WHERE character_id=encode($1,'hex')::uuid")
            .bind(character).bind(TRACK).bind(QUEST).execute(&mut *tx).await?;
    }
    if item {
        // Three exact donor items are retained as source definitions. Seed
        // existing custody so the real admission reads them on all four pads.
        let goods = id(231 + u8::try_from(index)?);
        let bag = id(241 + u8::try_from(index)?);
        for (identity,key) in [(bag,"oteryn:item.tibia.i2854"),(goods,PARTIES[index].3)] {
            sqlx::query("INSERT INTO game_item_instances
                (item_instance_id,world_id,definition_family,definition_production_key,
                 definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,
                 'definition-r1',1,1,encode($1,'hex')::uuid)")
                .bind(identity.as_slice()).bind(world).bind(key).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO game_item_container_slots
            (character_id,item_instance_id,world_id,placed_transaction_id)
            VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,
                   encode($3,'hex')::uuid,encode($2,'hex')::uuid)")
            .bind(character).bind(bag.as_slice()).bind(world).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries
            (item_instance_id,world_id,character_id,parent_item_instance_id,
             placement_ordinal,placed_transaction_id)
            VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,
                   encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($1,'hex')::uuid)")
            .bind(goods.as_slice()).bind(world).bind(character).bind(bag.as_slice())
            .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub(super) async fn move_member(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &AdmittedSession,
    position: Position,
) -> TestResult {
    let actor = member.runtime_actor.ok_or("entry fixture actor")?;
    relocate(owner, actor, member.game_session_id, (position.x, position.y, position.floor.unsigned_abs())).await
}

pub(super) async fn assert_entry(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [AdmittedSession],
    map: &BundleMap,
    pool: &sqlx::PgPool,
) -> TestResult {
    for (index, member) in members.iter().enumerate() {
        if index != 3 {
            seed(owner,pool,member,index,true,true).await?;
        }
        move_member(owner,member,PARTIES[index].0).await?;
    }
    assert_eq!(use_target(owner,&mut members[3],&target(map,LEVER)?).await?,UseDisposition::Rejected);
    assert_eq!(appearance(map,LEVER)?,2772);
    seed(owner,pool,&members[3],3,true,false).await?;
    assert_eq!(use_target(owner,&mut members[3],&target(map,LEVER)?).await?,UseDisposition::Rejected);
    assert_eq!(appearance(map,LEVER)?,2772);
    let cues = map.facts.movement_speech_since(0);
    assert_eq!(cues.iter().map(|(_,cue)|cue.text.as_str()).collect::<Vec<_>>(),[INELIGIBLE,INELIGIBLE]);

    seed(owner,pool,&members[3],3,false,true).await?;
    assert_party_admitted(owner,members,map).await?;

    // Returning the operator outside while others remain proves the spectator
    // branch before the circuit has any timer.
    move_member(owner,&members[3],START).await?;
    assert_eq!(use_target(owner,&mut members[3],&target(map,LEVER)?).await?,UseDisposition::Committed);
    assert_occupied(owner,&mut members[3],map).await?;
    move_member(owner,&members[3],PARTIES[3].1).await?;
    Ok(())
}

pub(super) async fn assert_occupied(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    operator: &mut AdmittedSession,
    map: &BundleMap,
) -> TestResult {
    assert_eq!(use_target(owner,operator,&target(map,LEVER)?).await?,UseDisposition::Occupied);
    assert_eq!(appearance(map,LEVER)?,2772);
    assert_eq!(map.facts.movement_speech_since(0).last().ok_or("occupied room cue")?.1.text,OCCUPIED);
    Ok(())
}

pub(super) async fn assert_party_admitted(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [AdmittedSession],
    map: &BundleMap,
) -> TestResult {
    assert_eq!(use_target(owner,&mut members[3],&target(map,LEVER)?).await?,UseDisposition::Committed);
    assert_eq!(appearance(map,LEVER)?,2773);
    let runtime = owner.runtime.lock().await;
    for (member, (_,destination,_,_)) in members.iter().zip(PARTIES) {
        assert_eq!(runtime.read_actor_position(member.runtime_actor.ok_or("entry actor")?)
            .map_err(debug)?.position(),local(destination));
    }
    Ok(())
}
