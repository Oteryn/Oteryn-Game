//! Native Lion Strength and Tears: an exact carried offering, source state and15s world lifecycle.
use super::{AdmittedSession,ComposedFreshAdmission,TestResult,fixture::{Harness,debug}};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::map::view::{MapFacts,placement_key};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback,Position,Offering};
use oteryn_protocol_oteryn::world_object::{UseDisposition,WorldObjectTarget};
const NAME:&str="m07/lion-strength";
const SOURCE:Position=Position{x:33137,y:32291,floor:-8};

#[test]
fn m07_lion_strength_postgres_exact_consumption_counter_and_pagoda_reset()->TestResult {
    run(NAME,SOURCE)
}
#[test]
fn m07_lion_tears_postgres_exact_consumption_counter_and_pagoda_reset()->TestResult {
    run("m07/lion-tears",Position{x:33154,y:32279,floor:-8})
}
#[test]
fn m07_lion_well_postgres_repeatable_exact_reward_and_command_replay()->TestResult {
    run("m07/lion-well",Position{x:33137,y:32351,floor:-6})
}
fn run(name:&'static str,source:Position)->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    let started=std::time::Instant::now();eprintln!("POSTGRES_CASE_STARTED {name}");
    let result=(||->TestResult {
        std::thread::Builder::new().stack_size(256<<20).spawn(move ||super::quest_world_callbacks::scenario(name,source,false).map_err(|e|e.to_string()))?
            .join().map_err(|_|"M07 Lion worker panicked")??;Ok(())
    })();
    eprintln!("POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",started.elapsed(),result.is_ok());result
}
pub(super) fn start(name:&str)->Position {
    if name=="m07/lion-well" {Position{x:33136,y:32351,floor:-6}}
    else if name=="m07/lion-tears" {Position{x:33155,y:32278,floor:-8}}
    else {Position{x:33136,y:32291,floor:-8}}
}

pub(super) fn world(native:&NativeGameplayState,name:&str)->TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{NativeCrop,ExpectedTile,native_scene_with_appearances};
    if name=="m07/lion-well" {
        return native_scene_with_appearances(native,vec![NativeCrop{
            bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z06-x129-y126.b3")),
            region:(6,129,126),bounds:(33135,32350,33139,32353),start:(33136,32351),
            expected:vec![
                ExpectedTile{position:(33137,32351),ids:vec![4409,21394],counted_entry:None},
                ExpectedTile{position:(33136,32351),ids:vec![4407],counted_entry:None},
            ],
        }],&[],&[]);
    }
    if name=="m07/lion-tears" {
        return native_scene_with_appearances(native,vec![NativeCrop{
            bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z08-x129-y126.b3")),
            region:(8,129,126),bounds:(33153,32277,33157,32281),start:(33155,32278),
            expected:vec![
                ExpectedTile{position:(33154,32279),ids:vec![19272,6481,21391],counted_entry:None},
                ExpectedTile{position:(33156,32279),ids:vec![499,2074],counted_entry:None},
                ExpectedTile{position:(33155,32278),ids:vec![499],counted_entry:None},
            ],
        }],&[],&[2075]);
    }
    native_scene_with_appearances(native,vec![NativeCrop{
        bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z08-x129-y126.b3")),
        region:(8,129,126),bounds:(33133,32288,33138,32292),start:(33136,32291),
        expected:vec![
            ExpectedTile{position:(33137,32291),ids:vec![19271,21390],counted_entry:None},
            ExpectedTile{position:(33134,32289),ids:vec![499,2074],counted_entry:None},
            ExpectedTile{position:(33136,32291),ids:vec![499],counted_entry:None},
        ],
    }],&[],&[2075])
}

pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,
    harness:&Harness,fence:CurrentCharacterGameplayFence,callback:&Callback)->TestResult {
    if callback.effects.item_reward_per_use {
        return Box::pin(well_exercise(owner,admitted,harness,fence,callback)).await;
    }
    let version:String=sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&harness.pool).await?;
    eprintln!("POSTGRES_DATABASE_READY {} version={version}",callback.key);
    let native=callback.selector.native.as_ref().ok_or("native offering selector")?;
    let position=native.position;
    let passage=callback.passages.first().ok_or("pagoda passage")?;
    let offer=callback.item_on_target.as_ref().ok_or("offering input")?;
    let map=owner.bundle_map().ok_or("map")?;
    let source=placement_key(crate::map::overlay::TilePos{x:position.x,y:position.y,floor:position.floor},native.ordinal).ok_or("source key")?;
    let pagoda=placement_key(crate::map::overlay::TilePos{x:passage.position.x,y:passage.position.y,floor:passage.position.floor},1).ok_or("pagoda key")?;
    let mut placement=map.overlay.base().digest().to_vec();placement.extend_from_slice(&source.to_be_bytes());
    let payload=|expected_revision|oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget{placement:placement.clone(),expected_revision}).map_err(debug);
    let reply=super::quest_world_callbacks::wire(owner,admitted,1,2,payload(0)?).await?;
    assert_eq!(oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,UseDisposition::Rejected);
    assert_eq!(counts(harness).await?,(0,0));
    let input=Offering{position,item:offer.item.clone(),count:offer.count};
    super::quest_world_callbacks::input_item(owner,fence.item_fence(),&input,71,true).await?;
    assert_eq!(carried(harness,&offer.item).await?,1);
    // A changed remote source must refuse before the exact carried input is charged.
    {
        let map=owner.bundle_map().ok_or("map")?;
        let changed=map.facts.quest_callback_replacement(pagoda,2075).ok_or("lit pagoda")?;
        assert!(map.facts.apply_quest_callback(pagoda,0,changed,&[],&[],false,None));
    }
    pace(owner)?;
    let reply=super::quest_world_callbacks::wire(owner,admitted,2,2,payload(0)?).await?;
    assert_eq!(oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,UseDisposition::Rejected);
    assert_eq!(carried(harness,&offer.item).await?,1);
    assert_eq!(counts(harness).await?,(0,0));
    {
        let map=owner.bundle_map().ok_or("map")?;
        let restored=map.facts.quest_callback_replacement(pagoda,2074).ok_or("unlit pagoda")?;
        assert!(map.facts.apply_quest_callback(pagoda,1,restored,&[],&[],false,None));
    }
    pace(owner)?;
    let reply=super::quest_world_callbacks::wire(owner,admitted,3,2,payload(0)?).await?;
    assert_eq!(oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,UseDisposition::Committed);
    assert_eq!(carried(harness,&offer.item).await?,0);
    let state=owner.root.read_character_quest_state(owner.character, fence.character_id).await.map_err(debug)?;
    assert_eq!(state.tracks().get(&callback.storage[0].track).copied(),Some(1));
    assert_eq!(state.tracks().get(&callback.state_ops[0].track).copied(),Some(2),"first source offering starts at2");
    let committed=counts(harness).await?;
    assert_eq!(committed.0,1);assert_eq!(committed.1,1);
    {
        let map=owner.bundle_map().ok_or("map")?;
        let due=map.facts.next_movement_decay_time().ok_or("pagoda deadline")?;
        assert_eq!(map.facts.expire_movement_items(due-1),0);
        assert_eq!(map.facts.expire_movement_items(due),1);
        assert_eq!(map.facts.object_revision(pagoda),4);
    }
    let current=owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"current fence")?.ok_or("current fence")?;
    super::quest_world_callbacks::input_item(owner,current.item_fence(),&input,72,true).await?;
    pace(owner)?;
    let reply=super::quest_world_callbacks::wire(owner,admitted,4,2,payload(1)?).await?;
    assert_eq!(oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,UseDisposition::Rejected);
    assert_eq!(carried(harness,&offer.item).await?,1,"completed trial never charges a second input");
    assert_eq!(counts(harness).await?,committed);
    Ok(())
}
fn pace(owner:&mut ComposedFreshAdmission<'_, '_, '_>)->TestResult {
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(1)).ok_or("clock")?;Ok(())
}
async fn carried(harness:&Harness,item:&str)->TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1").bind(item).fetch_one(&harness.pool).await?)
}
async fn counts(harness:&Harness)->TestResult<(i64,i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_callback_consumptions)").fetch_one(&harness.pool).await?)
}

async fn well_exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,
    harness:&Harness,fence:CurrentCharacterGameplayFence,callback:&Callback)->TestResult {
    let version:String=sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&harness.pool).await?;
    eprintln!("POSTGRES_DATABASE_READY {} version={version}",callback.key);
    let native=callback.selector.native.as_ref().ok_or("native well")?;
    let map=owner.bundle_map().ok_or("map")?;
    let source=placement_key(crate::map::overlay::TilePos{x:native.position.x,y:native.position.y,floor:native.position.floor},native.ordinal).ok_or("well key")?;
    let mut placement=map.overlay.base().digest().to_vec();placement.extend_from_slice(&source.to_be_bytes());
    let payload=|expected_revision|oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget{placement:placement.clone(),expected_revision}).map_err(debug);
    let item="oteryn:item.tibia.i21466";
    assert_eq!(carried(harness,item).await?,0);
    let mut first=None;
    for (index,(id,revision,quantity,disposition)) in [
        (1,0,1,UseDisposition::Committed),
        (1,0,1,UseDisposition::Committed),
        (2,99,1,UseDisposition::StaleState),
        (3,0,2,UseDisposition::Committed),
    ].into_iter().enumerate() {
        pace(owner)?;
        // Keep the large connection future outside this scenario's coroutine frame.
        let reply=Box::pin(super::quest_world_callbacks::wire(owner,admitted,id,2,payload(revision)?)).await?;
        assert_eq!(oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,disposition);
        assert_eq!(carried(harness,item).await?,quantity,"producer command {index}");
        assert_eq!(counts(harness).await?,(quantity,0),"one receipt per fresh grant; replay/stale source add none");
        if index==1 {assert_eq!(&reply,first.as_ref().ok_or("first result")?);}
        if index==0 {first=Some(reply);}
    }
    let state=owner.root.read_character_quest_state(owner.character,fence.character_id).await.map_err(debug)?;
    assert_eq!(state.tracks().get(&callback.state_ops[0].track).copied(),Some(1),"well starts but never advances the offering counter");
    Ok(())
}
