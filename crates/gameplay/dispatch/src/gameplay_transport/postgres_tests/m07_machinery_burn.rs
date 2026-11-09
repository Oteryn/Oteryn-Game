//! Native Machinery trial: exact reusable tool, live level and once-per-site credit.
use super::{AdmittedSession,ComposedFreshAdmission,TestResult,fixture::{Harness,debug}};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::map::view::{MapFacts,placement_key};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback,NativeSelector,Offering,Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition,WorldObjectTarget};

const NAME:&str="m07/machinery-ballista-01";
const SOURCE:Position=Position{x:32824,y:31708,floor:-7};
pub(super) fn start(name:&str)->Position {
    if name.starts_with("m07/machinery-catapult-") {return super::m07_machinery_catapults::start();}
    Position{x:32824,y:31709,floor:-7}
}

#[test]
fn m07_machinery_postgres_ballista_retains_tool_checks_level_and_resets()->TestResult {
    run(NAME,SOURCE)
}
#[test]
fn m07_machinery_postgres_ballista_refuses_level49_without_credit_or_tool_charge()->TestResult {
    run("m07/machinery-ballista-01-level49",SOURCE)
}
pub(super) fn run(name:&'static str,source:Position)->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    let begun=std::time::Instant::now();eprintln!("POSTGRES_CASE_STARTED {name}");
    let result=(||->TestResult {
        std::thread::Builder::new().stack_size(256<<20)
            .spawn(move ||super::quest_world_callbacks::scenario(name,source,false).map_err(|e|e.to_string()))?
            .join().map_err(|_|"Machinery burn worker panicked")??;Ok(())
    })();
    eprintln!("POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",begun.elapsed(),result.is_ok());result
}

pub(super) fn world(native:&NativeGameplayState,name:&str)->TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{NativeCrop,ExpectedTile,native_scene_with_appearances};
    if name.starts_with("m07/machinery-catapult-") {return super::m07_machinery_catapults::world(native);}
    native_scene_with_appearances(native,vec![NativeCrop{
        bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z07-x128-y123.b3")),
        region:(7,128,123),bounds:(32822,31706,32827,31711),start:(32824,31709),
        expected:vec![
            ExpectedTile{position:(32824,31708),ids:vec![4515,5697],counted_entry:None},
            ExpectedTile{position:(32824,31707),ids:vec![4515,5695],counted_entry:None},
            ExpectedTile{position:(32823,31708),ids:vec![4518,5696],counted_entry:None},
            ExpectedTile{position:(32824,31709),ids:vec![4515],counted_entry:None},
        ],
    }],&[],&[2113])
}

pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,
    harness:&Harness,fence:CurrentCharacterGameplayFence,callback:&Callback)->TestResult {
    let target=callback.selector.native.as_ref().ok_or("native machinery target")?;
    let pos=crate::map::overlay::TilePos{x:target.position.x,y:target.position.y,floor:target.position.floor};
    let source_key=placement_key(pos,target.ordinal).ok_or("source key")?;
    let source_id={
        let map=owner.bundle_map().ok_or("map")?;
        let tile=map.overlay.base().tile(pos.x,pos.y,pos.floor).ok_or("source tile")?;
        *tile.ids().iter().zip(tile.depths()).filter(|(_,depth)|**depth==0)
            .nth(usize::from(target.ordinal)).ok_or("source ordinal")?.0
    };
    let version:String=sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&harness.pool).await?;
    let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
    eprintln!("POSTGRES_DATABASE_READY {} version={version} level={level}",callback.key);
    assert_eq!(Box::pin(use_source(owner,admitted,1,0,target)).await?,UseDisposition::Rejected);
    assert_eq!(receipts(harness).await?,0);
    let tool=Offering{position:target.position,item:"oteryn:item.tibia.i5467".into(),count:1};
    super::quest_world_callbacks::input_item(owner,fence.item_fence(),&tool,91,true).await?;
    assert_eq!(carried(harness).await?,1);
    if level==49 {
        assert_eq!(Box::pin(use_source(owner,admitted,2,0,target)).await?,UseDisposition::Rejected,
            "a real level49 character with the exact tool cannot earn level50 credit");
        assert_eq!(receipts(harness).await?,0);assert_eq!(carried(harness).await?,1);
        assert_eq!(state(owner,fence,callback).await?,(None,None));
        assert_eq!(owner.bundle_map().ok_or("map")?.facts.object_revision(source_key),0);
        return Ok(());
    }
    assert_eq!(level,50);
    let rear_definition=callback.world_requirements.get(1).ok_or("required rear part")?;
    let rear={
        let map=owner.bundle_map().ok_or("map")?;
        let p=rear_definition.position;
        let pos=crate::map::overlay::TilePos{x:p.x,y:p.y,floor:p.floor};
        let tile=map.overlay.base().tile(p.x,p.y,p.floor).ok_or("rear tile")?;
        let mut found=None;
        for (ordinal,(id,_)) in tile.ids().iter().zip(tile.depths()).filter(|(_,depth)|**depth==0).enumerate() {
            let facts=map.facts.base_entry(pos,u8::try_from(ordinal)?,*id).ok_or("rear facts")?;
            if facts.appearance_id==rear_definition.appearance {
                assert!(found.is_none(),"rear part must be unambiguous");
                found=placement_key(pos,u8::try_from(ordinal)?);
            }
        }
        found.ok_or("rear key")?
    };
    {
        let map=owner.bundle_map().ok_or("map")?;
        let changed=map.facts.quest_callback_replacement(rear,2113).ok_or("changed machine part")?;
        assert!(map.facts.apply_quest_callback(rear,0,changed,&[],&[],false,None));
    }
    assert_eq!(Box::pin(use_source(owner,admitted,2,0,target)).await?,UseDisposition::Rejected,
        "every exact machine part is required before credit");
    assert_eq!(receipts(harness).await?,0);assert_eq!(carried(harness).await?,1);
    {
        let map=owner.bundle_map().ok_or("map")?;
        let restored=map.facts.quest_callback_replacement(rear,rear_definition.appearance).ok_or("original rear")?;
        assert!(map.facts.apply_quest_callback(rear,1,restored,&[],&[],false,None));
    }
    assert_eq!(Box::pin(use_source(owner,admitted,3,0,target)).await?,UseDisposition::Committed);
    assert_eq!(state(owner,fence,callback).await?,(Some(1),Some(1)));
    assert_eq!(receipts(harness).await?,1);assert_eq!(carried(harness).await?,1);
    assert_eq!(Box::pin(use_source(owner,admitted,3,0,target)).await?,UseDisposition::Committed);
    assert_eq!(receipts(harness).await?,1);
    {
        let map=owner.bundle_map().ok_or("map")?;
        let key=source_key;
        assert_eq!(map.facts.object_revision(key),1);
        assert_eq!(map.facts.base_entry(pos,target.ordinal,source_id).ok_or("lit front")?.appearance_id,2113);
        let due=map.facts.next_movement_decay_time().ok_or("source fire reset")?;
        assert_eq!(map.facts.expire_movement_items(due-1),0);
        assert_eq!(map.facts.expire_movement_items(due),1);
        assert_eq!(map.facts.object_revision(key),2);
        assert_eq!(map.facts.base_entry(pos,target.ordinal,source_id).ok_or("restored front")?.appearance_id,target.appearance);
    }
    assert_eq!(Box::pin(use_source(owner,admitted,4,2,target)).await?,UseDisposition::Rejected,
        "shared restoration never clears personal site credit");
    assert_eq!(state(owner,fence,callback).await?,(Some(1),Some(1)));
    assert_eq!(receipts(harness).await?,1);assert_eq!(carried(harness).await?,1);
    Ok(())
}

async fn use_source(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,id:u64,revision:u64,target:&NativeSelector)->TestResult<UseDisposition> {
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(1)).ok_or("clock")?;
    let map=owner.bundle_map().ok_or("map")?;
    let key=placement_key(crate::map::overlay::TilePos{x:target.position.x,y:target.position.y,floor:target.position.floor},target.ordinal).ok_or("source key")?;
    let mut placement=map.overlay.base().digest().to_vec();placement.extend_from_slice(&key.to_be_bytes());
    let payload=oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget{placement,expected_revision:revision}).map_err(debug)?;
    let reply=Box::pin(super::quest_world_callbacks::wire(owner,admitted,id,2,payload)).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
async fn carried(h:&Harness)->TestResult<i64>{
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i5467' AND i.lifecycle=1").fetch_one(&h.pool).await?)
}
async fn receipts(h:&Harness)->TestResult<i64>{
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts").fetch_one(&h.pool).await?)
}
async fn state(owner:&ComposedFreshAdmission<'_, '_, '_>,fence:CurrentCharacterGameplayFence,callback:&Callback)->TestResult<(Option<i64>,Option<i64>)>{
    let state=owner.root.read_character_quest_state(owner.character,fence.character_id).await.map_err(debug)?;
    Ok((state.tracks().get(&callback.storage[0].track).copied(),state.tracks().get(&callback.state_ops[0].track).copied()))
}
