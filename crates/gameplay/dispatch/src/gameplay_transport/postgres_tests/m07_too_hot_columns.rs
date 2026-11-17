//! Native column repair trial: exact retained chisel, live level, personal credit.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::{Harness, debug}};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::map::view::placement_key;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const SOURCE: Position = Position { x:32593, y:31841, floor:-9 };
pub(super) fn start()->Position {Position{x:32593,y:31842,floor:-9}}

#[test]
fn m07_too_hot_postgres_column_retains_chisel_and_credits_once()->TestResult {
    run("m07/too-hot-column-01")
}
#[test]
fn m07_too_hot_postgres_column_refuses_level79_without_credit()->TestResult {
    run("m07/too-hot-column-01-level79")
}
fn run(name:&'static str)->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    let begun=std::time::Instant::now();eprintln!("POSTGRES_CASE_STARTED {name}");
    let result=(||->TestResult {
        std::thread::Builder::new().stack_size(256<<20)
            .spawn(move ||super::quest_world_callbacks::scenario(name,SOURCE,false).map_err(|e|e.to_string()))?
            .join().map_err(|_|"Too Hot column worker panicked")??;Ok(())
    })();
    eprintln!("POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",begun.elapsed(),result.is_ok());result
}
pub(super) fn world(native:&NativeGameplayState)->TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{NativeCrop,ExpectedTile,native_scene_with_appearances};
    native_scene_with_appearances(native,vec![NativeCrop{
        bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z09-x127-y124.b3")),
        region:(9,127,124),bounds:(32592,31840,32594,31843),start:(32593,31842),
        expected:vec![
            ExpectedTile{position:(32593,31841),ids:vec![409,36541],counted_entry:None},
            ExpectedTile{position:(32593,31842),ids:vec![409],counted_entry:None},
        ],
    }],&[],&[])
}
pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,
    h:&Harness,fence:CurrentCharacterGameplayFence,callback:&Callback)->TestResult {
    let version:String=sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&h.pool).await?;
    let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    eprintln!("POSTGRES_DATABASE_READY {} version={version} level={level}",callback.key);
    assert_eq!(Box::pin(use_source(owner,admitted,1,0)).await?,UseDisposition::Rejected);
    assert_eq!(counts(h).await?,(0,0));
    let tool=Offering{position:SOURCE,item:"oteryn:item.tibia.i36543".into(),count:1};
    super::quest_world_callbacks::input_item(owner,fence.item_fence(),&tool,91,true).await?;
    assert_eq!(counts(h).await?,(1,0));
    let outcome=Box::pin(use_source(owner,admitted,2,0)).await?;
    let state=owner.root.read_character_quest_state(owner.character,fence.character_id).await.map_err(debug)?;
    if level==79 {
        assert_eq!(outcome,UseDisposition::Rejected);assert_eq!(counts(h).await?,(1,0));
        assert!(!state.tracks().contains_key(&callback.storage[0].track));
        assert!(!state.tracks().contains_key(&callback.state_ops[0].track));
        return Ok(());
    }
    assert_eq!(level,80);assert_eq!(outcome,UseDisposition::Committed);
    assert_eq!(state.tracks().get(&callback.storage[0].track),Some(&1));
    assert_eq!(state.tracks().get(&callback.state_ops[0].track),Some(&1));
    assert_eq!(counts(h).await?,(1,1));
    assert_eq!(Box::pin(use_source(owner,admitted,2,0)).await?,UseDisposition::Committed);
    assert_eq!(counts(h).await?,(1,1));
    assert_eq!(Box::pin(use_source(owner,admitted,3,1)).await?,UseDisposition::Rejected);
    assert_eq!(counts(h).await?,(1,1));
    let state=owner.root.read_character_quest_state(owner.character,fence.character_id).await.map_err(debug)?;
    assert_eq!(state.tracks().get(&callback.state_ops[0].track),Some(&1));
    Ok(())
}
async fn use_source(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,id:u64,revision:u64)->TestResult<UseDisposition> {
    let map=owner.bundle_map().ok_or("map")?;
    let key=placement_key(crate::map::overlay::TilePos{x:SOURCE.x,y:SOURCE.y,floor:SOURCE.floor},1).ok_or("column key")?;
    let mut placement=map.overlay.base().digest().to_vec();placement.extend_from_slice(&key.to_be_bytes());
    let payload=oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget{placement,expected_revision:revision}).map_err(debug)?;
    let reply=Box::pin(super::quest_world_callbacks::wire(owner,admitted,id,2,payload)).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
async fn counts(h:&Harness)->TestResult<(i64,i64)> {
    let tool=sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i36543' AND i.lifecycle=1").fetch_one(&h.pool).await?;
    let receipts=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts").fetch_one(&h.pool).await?;
    Ok((tool,receipts))
}
