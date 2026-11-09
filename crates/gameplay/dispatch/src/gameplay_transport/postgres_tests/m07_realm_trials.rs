//! M07 chosen Realm control: retained key, complete multi-floor census and two timers.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::{Harness,debug}};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::MovementLocalPosition;
use crate::map::view::{MapFacts,placement_key};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback,Position,Offering};
use oteryn_protocol_oteryn::world_object::{UseDisposition,WorldObjectTarget};
const NAME:&str="m07/realm-trial";
const BEGIN:Position=Position{x:33080,y:32582,floor:-3};
const CONTROL:Position=Position{x:33080,y:32583,floor:-3};
pub(super) fn start()->Position {Position{x:33079,y:32583,floor:-3}}
#[test]
fn m07_realm_postgres_retained_key_multifloor_clear_and_independent_resets()->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    let started=std::time::Instant::now();eprintln!("POSTGRES_CASE_STARTED {NAME}");
    let result=(||->TestResult {
        std::thread::Builder::new().stack_size(256<<20).spawn(||super::quest_world_callbacks::scenario(NAME,CONTROL,false).map_err(|e|e.to_string()))?
            .join().map_err(|_|"Realm trial worker panicked")??;Ok(())
    })();
    eprintln!("POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",started.elapsed(),result.is_ok());result
}
pub(super) fn world(native:&NativeGameplayState)->TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{NativeCrop,ExpectedTile,native_scene_with_appearances};
    native_scene_with_appearances(native,vec![
        NativeCrop{
            bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z03-x129-y127.b3")),
            region:(3,129,127),bounds:(33073,32580,33084,32593),start:(33079,32583),
            expected:vec![
                ExpectedTile{position:(33080,32582),ids:vec![104,1772,1823,2772],counted_entry:None},
                ExpectedTile{position:(33080,32583),ids:vec![430,2124],counted_entry:None},
                ExpectedTile{position:(33079,32583),ids:vec![104],counted_entry:None},
                ExpectedTile{position:(33075,32591),ids:vec![104,2129],counted_entry:None},
            ],
        },
        NativeCrop{
            bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z05-x129-y127.b3")),
            region:(5,129,127),bounds:(33087,32582,33089,32584),start:(33088,32583),
            expected:vec![ExpectedTile{position:(33088,32583),ids:vec![103],counted_entry:None}],
        },
    ],&[],&[2773])
}
pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,
    harness:&Harness,fence:CurrentCharacterGameplayFence,callback:&Callback)->TestResult {
    let version:String=sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&harness.pool).await?;
    eprintln!("POSTGRES_DATABASE_READY {NAME} version={version}");
    let key=Offering{position:BEGIN,item:"oteryn:item.tibia.i14682".into(),count:1};
    assert_eq!(Box::pin(use_source(owner,admitted,1,BEGIN,3,0)).await?,UseDisposition::Rejected);
    assert_eq!(receipts(harness).await?,0);
    super::quest_world_callbacks::input_item(owner,fence.item_fence(),&key,81,true).await?;
    assert_eq!(key_count(harness).await?,1);
    assert_eq!(Box::pin(use_source(owner,admitted,2,CONTROL,1,0)).await?,UseDisposition::Rejected,"a key is not a started trial");
    assert_eq!(receipts(harness).await?,0);
    assert_eq!(Box::pin(use_source(owner,admitted,3,BEGIN,3,0)).await?,UseDisposition::Committed);
    assert_eq!(phase(owner,fence,callback).await?,Some(1));
    let blocker={
        let mut runtime=owner.runtime.lock().await;
        owner.active_generation.and_then(|g|g.native_gameplay()).ok_or("native policies")?
            .install_companion_policies(&mut runtime).map_err(debug)?;
        let creature="oteryn:creature.dragon";
        let health=runtime.companion_policy(creature).map_err(debug)?.maximum_health;
        let actor=runtime.admit_encounter_actor(creature,health,MovementLocalPosition{x:33088,y:32583,floor:5}).map_err(debug)?;
        runtime.install_creature_policy(actor,creature).map_err(debug)?;actor
    };
    assert_eq!(Box::pin(use_source(owner,admitted,4,CONTROL,1,0)).await?,UseDisposition::Rejected,"live guardian on another opted-in floor blocks clearance");
    assert_eq!(phase(owner,fence,callback).await?,Some(1));
    assert_eq!(receipts(harness).await?,1);assert_eq!(key_count(harness).await?,1);
    assert!(owner.runtime.lock().await.kill_creature_for_test(blocker));
    assert_eq!(Box::pin(use_source(owner,admitted,5,CONTROL,1,0)).await?,UseDisposition::Committed);
    assert_eq!(phase(owner,fence,callback).await?,Some(2));
    assert_eq!(receipts(harness).await?,2);assert_eq!(key_count(harness).await?,1,"both native uses retain the exact key");
    assert_eq!(Box::pin(use_source(owner,admitted,5,CONTROL,1,0)).await?,UseDisposition::Committed,"historical command replays without another write");
    assert_eq!(receipts(harness).await?,2);
    {
        let map=owner.bundle_map().ok_or("map")?;
        let control=placement_key(crate::map::overlay::TilePos{x:CONTROL.x,y:CONTROL.y,floor:CONTROL.floor},1).ok_or("control key")?;
        let wall=placement_key(crate::map::overlay::TilePos{x:33075,y:32591,floor:-3},1).ok_or("wall key")?;
        let wall_due=map.facts.next_movement_decay_time().ok_or("wall reset")?;
        assert_eq!(map.facts.expire_movement_items(wall_due-1),0);
        assert_eq!(map.facts.expire_movement_items(wall_due),1);
        assert_eq!(map.facts.object_revision(wall),2);
        assert_eq!(map.facts.object_revision(control),1,"control retains its longer cooldown");
        let control_due=map.facts.next_movement_decay_time().ok_or("control reset")?;
        assert_eq!(control_due-wall_due,240_000_000);
        assert_eq!(map.facts.expire_movement_items(control_due-1),0);
        assert_eq!(map.facts.expire_movement_items(control_due),1);
        assert_eq!(map.facts.object_revision(control),2);
    }
    assert_eq!(Box::pin(use_source(owner,admitted,6,CONTROL,1,2)).await?,UseDisposition::Rejected,"completed personal trial cannot reopen on shared reset");
    assert_eq!(key_count(harness).await?,1);assert_eq!(receipts(harness).await?,2);
    Ok(())
}
async fn use_source(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,id:u64,position:Position,ordinal:u8,revision:u64)->TestResult<UseDisposition> {
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(1)).ok_or("clock")?;
    let map=owner.bundle_map().ok_or("map")?;
    let key=placement_key(crate::map::overlay::TilePos{x:position.x,y:position.y,floor:position.floor},ordinal).ok_or("source key")?;
    let mut placement=map.overlay.base().digest().to_vec();placement.extend_from_slice(&key.to_be_bytes());
    let payload=oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget{placement,expected_revision:revision}).map_err(debug)?;
    let reply=Box::pin(super::quest_world_callbacks::wire(owner,admitted,id,2,payload)).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
async fn key_count(harness:&Harness)->TestResult<i64>{
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i14682' AND i.lifecycle=1").fetch_one(&harness.pool).await?)
}
async fn receipts(harness:&Harness)->TestResult<i64>{
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts").fetch_one(&harness.pool).await?)
}
async fn phase(owner:&ComposedFreshAdmission<'_, '_, '_>,fence:CurrentCharacterGameplayFence,callback:&Callback)->TestResult<Option<i64>>{
    Ok(owner.root.read_character_quest_state(owner.character,fence.character_id).await.map_err(debug)?.tracks().get(&callback.storage[0].track).copied())
}
