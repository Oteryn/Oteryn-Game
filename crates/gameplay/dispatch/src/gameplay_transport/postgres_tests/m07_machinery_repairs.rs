//! Atomic seven-instance native repair: no partial budget, occupied wall or replay charge.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::{Harness, debug}};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::MovementLocalPosition;
use crate::map::view::{MapFacts, placement_key};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, NativeSelector, Offering, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const NAME:&str="m07/machinery-repair-outpost-01";
const SOURCE:Position=Position{x:32797,y:31731,floor:-7};
pub(super) fn start()->Position {Position{x:32797,y:31732,floor:-7}}
#[test]
fn m07_machinery_postgres_repair_consumes_exact_budget_and_keeps_walls_solid()->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    let begun=std::time::Instant::now();eprintln!("POSTGRES_CASE_STARTED {NAME}");
    let result=(||->TestResult {
        std::thread::Builder::new().stack_size(256<<20)
            .spawn(||super::quest_world_callbacks::scenario(NAME,SOURCE,false).map_err(|e|e.to_string()))?
            .join().map_err(|_|"Machinery repair worker panicked")??;Ok(())
    })();
    eprintln!("POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",begun.elapsed(),result.is_ok());result
}
pub(super) fn world(native:&NativeGameplayState)->TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{NativeCrop, ExpectedTile, native_scene_with_appearances};
    const BYTES:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z07-x128-y123.b3"));
    let mut crops=vec![NativeCrop{bytes:BYTES,region:(7,128,123),bounds:(32797,31731,32798,31733),start:(32797,31732),expected:vec![
        ExpectedTile{position:(32797,31731),ids:vec![4515,7608,6280],counted_entry:None},
        ExpectedTile{position:(32797,31732),ids:vec![7593],counted_entry:None},
    ]}];
    for (x,y,ids) in [
        (32798,31732,vec![7598,6480,6283]),(32793,31735,vec![4515,6477,7607,6283]),
        (32798,31735,vec![7598,6483,6283]),(32793,31736,vec![4519,7607,6281]),
        (32798,31736,vec![7599,6281]),(32797,31737,vec![7593,6481,6280]),
    ] {crops.push(NativeCrop{bytes:BYTES,region:(7,128,123),bounds:(x,y,x+1,y+1),start:(x,y),expected:vec![ExpectedTile{position:(x,y),ids,counted_entry:None}]});}
    native_scene_with_appearances(native,crops,&[],&[1271,1277,2113])
}
pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,
    h:&Harness,fence:CurrentCharacterGameplayFence,callback:&Callback)->TestResult {
    let target=callback.selector.native.as_ref().ok_or("repair control")?;
    let version:String=sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&h.pool).await?;
    eprintln!("POSTGRES_DATABASE_READY {NAME} version={version}");
    assert_eq!(Box::pin(use_source(owner,admitted,1,0,target)).await?,UseDisposition::Rejected);
    for n in 91..97 {mint_kit(owner,fence,n).await?;}
    assert_eq!(counts(h).await?,(6,0,0));
    assert_eq!(Box::pin(use_source(owner,admitted,2,0,target)).await?,UseDisposition::Rejected,"six separate kits cannot pay a seven-kit repair");
    assert_eq!(counts(h).await?,(6,0,0),"under-budget refusal burns nothing");
    mint_kit(owner,fence,97).await?;
    let remote=callback.passages.get(1).ok_or("remote wall")?;
    for (command,position) in [(3,SOURCE),(4,remote.position)] {
        let blocker={
            let mut runtime=owner.runtime.lock().await;
            owner.active_generation.and_then(|g|g.native_gameplay()).ok_or("policies")?
                .install_companion_policies(&mut runtime).map_err(debug)?;
            let creature="oteryn:creature.dragon";
            let health=runtime.companion_policy(creature).map_err(debug)?.maximum_health;
            let actor=runtime.admit_encounter_actor(creature,health,MovementLocalPosition{x:i32::from(position.x),y:i32::from(position.y),floor:i16::from(position.floor).saturating_neg()}).map_err(debug)?;
            runtime.install_creature_policy(actor,creature).map_err(debug)?;actor
        };
        assert_eq!(Box::pin(use_source(owner,admitted,command,0,target)).await?,UseDisposition::Occupied,"never repair a wall onto an actor");
        assert_eq!(counts(h).await?,(7,0,0));
        owner.runtime.lock().await.remove_encounter_actor(blocker).map_err(debug)?;
    }
    let remote_key=key_for(owner,remote.position,remote.appearance)?;
    {
        let map=owner.bundle_map().ok_or("map")?;
        let wrong=map.facts.quest_callback_replacement(remote_key,2113).ok_or("wrong wall")?;
        assert!(map.facts.apply_quest_callback(remote_key,0,wrong,&[],&[],false,None));
    }
    assert_eq!(Box::pin(use_source(owner,admitted,5,0,target)).await?,UseDisposition::Rejected);
    assert_eq!(counts(h).await?,(7,0,0),"world mismatch must not charge the budget");
    {
        let map=owner.bundle_map().ok_or("map")?;
        let original=map.facts.quest_callback_replacement(remote_key,remote.appearance).ok_or("original wall")?;
        assert!(map.facts.apply_quest_callback(remote_key,1,original,&[],&[],false,None));
    }
    assert_eq!(Box::pin(use_source(owner,admitted,6,0,target)).await?,UseDisposition::Committed);
    assert_eq!(counts(h).await?,(0,1,7));
    let transactions:i64=sqlx::query_scalar("SELECT count(DISTINCT transaction_id) FROM game_quest_callback_consumptions").fetch_one(&h.pool).await?;
    assert_eq!(transactions,1,"all seven physical instances use one atomic burn");
    let state=owner.root.read_character_quest_state(owner.character,fence.character_id).await.map_err(debug)?;
    assert_eq!(state.tracks().get(&callback.storage[0].track),Some(&1));
    assert_eq!(state.tracks().get(&callback.state_ops[0].track),Some(&1));
    assert_eq!(state.tracks().get(&callback.state_ops[1].track),Some(&7));
    for passage in &callback.passages {
        let key=key_for(owner,passage.position,passage.replacement.ok_or("repaired appearance")?)?;
        let map=owner.bundle_map().ok_or("map")?;
        assert!(map.facts.object_revision(key)>0);
        assert!(map.facts.quest_callback_blocked(map.overlay.base(),crate::map::overlay::TilePos{x:passage.position.x,y:passage.position.y,floor:passage.position.floor},false),"repaired walls must block, including the clicked wall");
    }
    assert_eq!(Box::pin(use_source(owner,admitted,6,0,target)).await?,UseDisposition::Committed);
    assert_eq!(counts(h).await?,(0,1,7));
    // A later external damage projection must not erase persistent personal credit.
    for passage in &callback.passages {
        let key=key_for(owner,passage.position,passage.replacement.ok_or("repaired appearance")?)?;
        let map=owner.bundle_map().ok_or("map")?;
        let revision=map.facts.object_revision(key);
        let original=map.facts.quest_callback_replacement(key,passage.appearance).ok_or("damaged wall")?;
        assert!(map.facts.apply_quest_callback(key,revision,original,&[],&[(key,original,true)],false,None));
    }
    for n in 98..105 {mint_kit(owner,fence,n).await?;}
    assert_eq!(Box::pin(use_source(owner,admitted,7,2,target)).await?,UseDisposition::Rejected,"repeated physical damage cannot double-credit a personal site");
    assert_eq!(counts(h).await?,(7,1,7));
    Ok(())
}
async fn mint_kit(owner:&ComposedFreshAdmission<'_, '_, '_>,fence:CurrentCharacterGameplayFence,n:u64)->TestResult {
    super::quest_world_callbacks::input_item(owner,fence.item_fence(),&Offering{position:SOURCE,item:"oteryn:item.tibia.i7785".into(),count:1},n,true).await
}
fn key_for(owner:&ComposedFreshAdmission<'_, '_, '_>,position:Position,appearance:u16)->TestResult<u64> {
    let map=owner.bundle_map().ok_or("map")?;
    let pos=crate::map::overlay::TilePos{x:position.x,y:position.y,floor:position.floor};
    let tile=map.overlay.base().tile(pos.x,pos.y,pos.floor).ok_or("wall tile")?;
    let mut found=None;
    for (ordinal,(id,_)) in tile.ids().iter().zip(tile.depths()).filter(|(_,d)|**d==0).enumerate() {
        let ordinal=u8::try_from(ordinal)?;
        if map.facts.base_entry(pos,ordinal,*id).ok_or("wall facts")?.appearance_id==appearance {
            assert!(found.is_none(),"exactly one selected wall");found=placement_key(pos,ordinal);
        }
    }
    found.ok_or_else(||"wall key".into())
}
async fn use_source(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,id:u64,revision:u64,target:&NativeSelector)->TestResult<UseDisposition> {
    let map=owner.bundle_map().ok_or("map")?;
    let key=placement_key(crate::map::overlay::TilePos{x:target.position.x,y:target.position.y,floor:target.position.floor},target.ordinal).ok_or("source key")?;
    let mut placement=map.overlay.base().digest().to_vec();placement.extend_from_slice(&key.to_be_bytes());
    let payload=oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget{placement,expected_revision:revision}).map_err(debug)?;
    let reply=Box::pin(super::quest_world_callbacks::wire(owner,admitted,id,2,payload)).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
async fn counts(h:&Harness)->TestResult<(i64,i64,i64)> {
    Ok(sqlx::query_as("SELECT (SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i7785' AND i.lifecycle=1),(SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_callback_consumptions)").fetch_one(&h.pool).await?)
}
