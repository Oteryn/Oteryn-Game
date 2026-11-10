//! Native Hydra entry/lethal and personal ordinary-map egg custody.
use super::fixture::{Harness,debug};
use super::{AdmittedSession,ComposedFreshAdmission,TestResult};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_spatial::{COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,StepDirection,StepDisposition,decode_step_result,encode_step_intent};
use oteryn_protocol_oteryn::world_object::{UseDisposition,decode_use_result};
const TRACK:&str="oteryn:quest-progress/questpc7/hydra/programme";
const KILLS:&str="oteryn:quest-progress/questpc7/hydra/kills";
#[test]
fn questpc7_hydra_requires_personal_native_nest_pickup() -> TestResult {
 if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none(){return Ok(());}
 let started=std::time::Instant::now();
 let result=super::kill_reward_e2e::scenario_thread::run("questpc7-hydra",||super::quest_world_callbacks::scenario("questpc7-hydra",Position{x:33003,y:32665,floor:-3},false));
 eprintln!("QUESTPC7_HYDRA POSTGRES_CASE_COMPLETED duration={:?} success={}",started.elapsed(),result.is_ok());result
}
async fn value(owner:&ComposedFreshAdmission<'_, '_, '_>,a:&AdmittedSession,key:&str)->TestResult<i64>{
 let f=owner.current_quest_fence(a.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
 let s=owner.root.read_character_quest_state(owner.character,f.character_id).await.map_err(debug)?;
 Ok(s.tracks().get(key).copied().unwrap_or(-1))
}
async fn nest(owner:&ComposedFreshAdmission<'_, '_, '_>,a:&mut AdmittedSession)->TestResult<UseDisposition>{
 tokio::time::sleep(std::time::Duration::from_millis(210)).await;
 let target=super::questpc7_hoist_story::target(owner,Position{x:33004,y:32664,floor:-3},1063)?;
 let command=a.continuity.next_command_id;
 let reply=super::quest_world_callbacks::wire(owner,a,command,2,target).await?;
 decode_use_result(&reply).map_err(debug)
}
pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,a:&mut AdmittedSession,h:&Harness)->TestResult {
 let version:String=sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
 eprintln!("QUESTPC7_HYDRA POSTGRES_STARTED version={version}");
 let native=owner.active_generation.and_then(|g|g.native_gameplay()).ok_or("native")?;
 native.install_companion_policies(&mut *owner.runtime.lock().await).map_err(debug)?;
 let actor=a.runtime_actor.ok_or("actor")?;
 let f=owner.current_quest_fence(a.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
 owner.runtime.lock().await.bind_attacker_lease(actor,a.game_session_id,crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(f.character_id.as_bytes()).map_err(debug)?,f.character_lease_generation).map_err(debug)?).map_err(debug)?;
 let mut selected=a.continuity.selected_capabilities.as_slice().to_vec();selected.extend([3,4,6,11,15,18,25]);selected.sort_unstable();selected.dedup();
 let offered=crate::gameplay_transport::appearance_change::offer::offered(owner.offered_capabilities(),owner.appearance_available());
 a.continuity.selected_capabilities=crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered,&selected).ok_or("capabilities")?;
 assert_eq!(value(owner,a,TRACK).await?,-1,"no subject progress fixtures");
 let command=a.continuity.next_command_id;
 let reply=super::quest_world_callbacks::wire(owner,a,command,COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,encode_step_intent(StepDirection::East)).await?;
 assert_eq!(decode_step_result(&reply).map_err(debug)?,StepDisposition::Moved);
 assert_eq!(value(owner,a,TRACK).await?,1);
 assert_eq!(value(owner,a,KILLS).await?,0);
 assert_eq!(nest(owner,a).await?,UseDisposition::Rejected,"kill quota is authoritative");
 owner.refresh_quest_session(a.game_session_id,false).await;
 let guardian={let runtime=owner.runtime.lock().await;let found:Vec<_>=runtime.positioned_melee_census().map_err(debug)?.into_iter().filter(|(_,p,s)|s.is_none()&&p.position()==MovementLocalPosition{x:33005,y:32665,floor:3}).map(|(actor,_,_)|actor).collect();assert_eq!(found.len(),1,"one native Hydra guardian");found[0]};
 super::questpc7_native_deaths::lethal_creature(owner,a,guardian,7000,"oteryn:creature.hydra").await?;
 assert_eq!(value(owner,a,KILLS).await?,1);
 assert_eq!(nest(owner,a).await?,UseDisposition::Rejected,"a kill alone cannot replace personally picking an egg from the native nest");
 assert_eq!(value(owner,a,TRACK).await?,1);
 // An unrelated egg cannot substitute for a successful personal native-source pickup.
 let fence=owner.current_quest_fence(a.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
 super::quest_world_callbacks::input_item(owner,fence.item_fence(),&oteryn_gameplay_quests::quest::world_callbacks::Offering{position:Position{x:33003,y:32665,floor:-3},item:"oteryn:item.tibia.i4839".into(),count:1},8000,true).await?;
 assert_eq!(nest(owner,a).await?,UseDisposition::Rejected,"unrelated carried egg has no native nest pickup receipt");
 pick_native_egg(owner,a).await?;
 assert_eq!(super::questpc7_outfit_items::quantity(h,4839).await?,2,"one native egg plus one unrelated input; normal merging preserves the pickup milestone");
 let stacks:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i4839'").bind(super::fixture::id(41).as_slice()).bind(owner.world_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;assert_eq!(stacks,1,"the native pickup really merged into the unrelated carried stack");
 assert_eq!(nest(owner,a).await?,UseDisposition::Committed);
 assert_eq!(value(owner,a,TRACK).await?,2);
 assert_eq!(super::questpc7_outfit_items::quantity(h,4839).await?,2,"acknowledgement neither consumes nor manufactures eggs");
 let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.hydra_egg_quest'").bind(super::fixture::id(41).as_slice()).fetch_one(&h.pool).await?;assert!(completed);
 let claims:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key LIKE '%questpc7-hydra%'").fetch_one(&h.pool).await?;assert_eq!(claims,0,"ordinary native pickup is not a fabricated quest chest");
 Ok(())
}

async fn pick_native_egg(owner:&ComposedFreshAdmission<'_, '_, '_>,a:&AdmittedSession)->TestResult {
 use crate::gameplay_transport::item_view::{SessionItemView,ItemViewContinuity,ItemKey};
 use crate::gameplay_transport::world_map::{SessionMapView,MapUpdate};
 use crate::gameplay_transport::item_move::{item_move,ItemMoveStep};
 use crate::gameplay_transport::connection::UseCommand;
 use oteryn_protocol_oteryn::item_view::{ItemMoveIntent,ItemMoveDestination,ItemMoveSelection,ItemMoveOutcome,encode_item_move_intent_for};
 let actor=a.runtime_actor.ok_or("actor")?;
 let mut items=SessionItemView::resume(ItemViewContinuity::default()).with_equip_drop().with_container_tree().with_map_view();
 items.snapshot(owner.observe_character_inventory(actor,a.game_session_id).await.ok_or("inventory")?,None).map_err(debug)?;
 let mut view=SessionMapView::default();
 let Some(MapUpdate::Snapshot(snapshot))=owner.observe_world_map(actor,&mut view,&mut items).await.ok_or("map")?.map_err(debug)? else{return Err("map snapshot".into());};
 let snapshot=oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload).map_err(debug)?;
 assert_eq!(snapshot.header.reset_epoch,owner.reset_epoch);
 let map=owner.bundle_map().ok_or("bundle")?;
 // Resolve the exact native definition, independent of fixture stack order.
 use crate::map::view::MapFacts;
 let position=crate::map::overlay::TilePos{x:33004,y:32664,floor:-3};
 let native=owner.active_generation.and_then(|g|g.native_gameplay()).ok_or("native")?;
 let expected=native.item_index().and_then(|i|i.definition_ref("Item","oteryn:item.tibia.i4839","definition-r1")).ok_or("egg definition")?;
 let tile=map.overlay.base().tile(position.x,position.y,position.floor).ok_or("nest tile")?;
 let candidates=tile.ids().iter().zip(tile.depths()).filter(|(_,depth)|**depth==0).enumerate()
  .filter_map(|(ordinal,(id,_))|{
   let ordinal=u8::try_from(ordinal).ok()?;
   let fact=map.facts.base_entry(position,ordinal,*id)?;
   (fact.definition==oteryn_protocol_oteryn::world_map::MapDefinition::Item(expected)
    && fact.pickupable && !fact.bound).then_some(ordinal)
  }).collect::<Vec<_>>();
 assert_eq!(candidates.len(),1,"one exact movable native egg at the nest");
 let key=ItemKey::MapBase{bundle_digest:map.overlay.base().digest(),placement_key:oteryn_world_bundle::bundle::placement_key(-3,33004,32664,candidates[0]).ok_or("key")?,reset_epoch:owner.reset_epoch};
 let handle=items.table().handle(&key).ok_or("native egg map handle")?;
 let selection=ItemMoveSelection{equip_drop:true,container_tree:true,world_interaction:true};
 let payload=encode_item_move_intent_for(&ItemMoveIntent{source:handle,destination:ItemMoveDestination::MainBackpack},selection).map_err(debug)?;
 let fence=owner.current_quest_fence(a.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
 let command=UseCommand{game_session_id:a.game_session_id,command_id:10000+owner.reset_epoch*10,item_fence:Some(fence.item_fence())};
 assert!(matches!(item_move(owner,actor,command,&payload,selection,false,&mut items).await,ItemMoveStep::Result(ItemMoveOutcome::Moved,_)),"ordinary command9 native map pickup must succeed");
 assert!(matches!(item_move(owner,actor,command,&payload,selection,false,&mut items).await,ItemMoveStep::Result(ItemMoveOutcome::Moved,_)),"exact native pickup replay");
 assert_eq!(item_move(owner,actor,UseCommand{command_id:command.command_id+1,..command},&payload,selection,false,&mut items).await,ItemMoveStep::Result(ItemMoveOutcome::Stale,vec![]),"an ordinary map source is taken once per reset");
 Ok(())
}


pub(super) async fn replenishment(owner:&mut ComposedFreshAdmission<'_, '_, '_>,a:&mut AdmittedSession,h:&Harness)->TestResult {
 assert_eq!(owner.reset_epoch,1);
 assert_eq!(h.root.active_world_reset_epoch(owner.world_id).await?,Some(1));
 // Production boots each epoch with fresh ChannelSessionStates. This fixture
 // retains its owner, so discard the prior epoch's cached taken-origin view.
 owner.states.loop_ground.invalidate();
 let before:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.hydra_egg_quest'").bind(super::fixture::id(41).as_slice()).fetch_one(&h.pool).await?;
 pick_native_egg(owner,a).await?;
 assert_eq!(super::questpc7_outfit_items::quantity(h,4839).await?,3,"ordinary nest replenishes at the next active reset even after quest completion");
 let receipts:(i64,i64)=sqlx::query_as("SELECT count(*),count(DISTINCT reset_epoch) FROM game_map_item_mint_receipts WHERE character_id=encode($1,'hex')::uuid AND definition_production_key='oteryn:item.tibia.i4839'").bind(super::fixture::id(41).as_slice()).fetch_one(&h.pool).await?;assert_eq!(receipts,(2,2));
 assert_eq!(nest(owner,a).await?,UseDisposition::Rejected,"reset replenishes the map source, not the once-only personal programme");
 assert_eq!(value(owner,a,TRACK).await?,2);
 let after:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.hydra_egg_quest'").bind(super::fixture::id(41).as_slice()).fetch_one(&h.pool).await?;assert_eq!(after,before);
 eprintln!("QUESTPC7_HYDRA ACTIVE_EPOCH_REPLENISHMENT_CONFIRMED epochs=2 quest_completion_unchanged=true");
 Ok(())
}
