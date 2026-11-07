//! Six source entries: world agaric, each selected map fragment, and the complete fairy treasure.
use super::super::fixture::{Harness,debug,id,run};
use super::super::{AdmittedSession,ComposedFreshAdmission,Dependencies,TestResult,prepare_current_session};
use super::barriers::{progress,quantity,relocate};
use super::rakes::seed;
use super::acquisition::use_source;
use super::maps::append_fixture_input;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Passage,Position};
use oteryn_gameplay_quests::quest::{QuestComparison,QuestEffect,QuestEffectKind,QuestStateCatalogue,QuestTransition};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition,UseIntent,UseTarget,WorldObjectTarget,encode_item_use_intent};
use std::sync::Arc;

const STONE:&str="oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/grumpy_stone";
const WORLD:&str="oteryn:quest-transition/questpc3/threatened-treasure/";
const CARRIED:&str="oteryn:quest-reward/questpc3/threatened-map/compose-";
const PARTS:[&str;4]=["oteryn:item.tibia.i24943","oteryn:item.tibia.i24944","oteryn:item.tibia.i24945","oteryn:item.tibia.i24946"];
const MAP:&str="oteryn:item.tibia.i24947";
// Agaric is registered by Item25385 globally; its coordinate here is fixture geometry.
const AT:Position=Position{x:32355,y:32416,floor:-7};
const TREASURE:Position=Position{x:32363,y:32416,floor:-7};
const WRONG:Position=Position{x:32363,y:32417,floor:-7};
fn neighbor(at:Position)->Position{Position{x:at.x-1,..at}}
fn prior(value:i64)->String{format!("oteryn:quest-transition/fixture/threatened-treasure/prior{value}")}

#[test]
fn threatened_treasure_six_entries_use_exact_inputs_and_complete_rewards()->TestResult{
 if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none(){eprintln!("THREATENED_TREASURE POSTGRES_SKIPPED no_admin_url");return Ok(());}
 std::thread::Builder::new().stack_size(256<<20).spawn(||{
  let started=std::time::Instant::now();match std::panic::catch_unwind(scenario){
   Ok(result)=>{eprintln!("THREATENED_TREASURE POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());result.map_err(|e|e.to_string())},
   Err(panic)=>{eprintln!("THREATENED_TREASURE POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());std::panic::resume_unwind(panic)}
  }
 })?.join().map_err(|_|"treasure scenario panicked")??;Ok(())
}
fn catalogue()->TestResult<QuestStateCatalogue>{
 let source=oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
 let track=source.catalogue().track(STONE).ok_or("source stone track")?.clone();
 let mut transitions=Vec::new();
 for c in oteryn_gameplay_quests::quest::world_callbacks::load()?.into_iter().filter(|c|c.key.starts_with(WORLD)){
  transitions.push(source.catalogue().transition(&c.key).ok_or("world source transition")?.clone());
 }
 for c in oteryn_gameplay_quests::quest::reward_callbacks::load()?.into_iter().filter(|c|c.key.starts_with(CARRIED)){
  transitions.push(source.catalogue().transition(&c.key).ok_or("carried source transition")?.clone());
 }
 for value in [1,2,4]{transitions.push(QuestTransition{key:prior(value),quest:track.quest.clone(),effects:vec![QuestEffect{track:STONE.into(),from:QuestComparison::Any,effect:QuestEffectKind::Set(value)}],completes:false,experience:None});}
 QuestStateCatalogue::new("content-1",vec![track],transitions).map_err(|e|format!("{e:?}").into())
}
fn scenario()->TestResult{
 let catalogue=Arc::new(catalogue()?);
 run(async |admin|{
        let h=Harness::create_with_admission_seed(admin,"threatened_treasure",false,false).await?;
        prepare_current_session(&h).await?;
        let version:String=sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&h.pool).await?;
        eprintln!("THREATENED_TREASURE POSTGRES_CASE_STARTED server_version={version}");
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut deps=Dependencies::new()?;
        let (controller,activated)=super::super::death_ml_loss_composed::production_generation_with_pin(deps.admitted.world_id,"content-1")?;
        let generation=controller.active().ok_or("generation")?;
        let mut geometry=oteryn_gameplay_quests::quest::world_callbacks::load()?.into_iter().next().ok_or("geometry prototype")?;
        geometry.selector.position=Some(AT);
        geometry.selector.action=None;
        geometry.selector.unique=None;
        geometry.selector.appearance=Some(25385);
        geometry.lever=None;
        geometry.teleport=None;
        geometry.item_on_target=None;
        geometry.carried_use=None;
        geometry.offerings.clear();
        geometry.passages=vec![Passage { position:TREASURE,appearance:25380,replacement:None },Passage { position:WRONG,appearance:25380,replacement:None }];
        let world=super::super::quest_world_callbacks::world(&geometry,AT)?;
        let cells=world.movement_cells(activated.qualified_room().movement_cells()).map_err(debug)?;
        let start=neighbor(AT);
        let pin=ChannelContentPin::from_activation(deps.admitted.world_id,1,generation.identity().server_artifact_digest(),generation.identity().client_artifact_digest(),activated.frame_binding().digest(),world.map_revision_digest(),(i32::from(start.x),i32::from(start.y),-i16::from(start.floor)));
        let mut runtime=ChannelRuntimeV1::from_committed_assignment(deps.admitted.world_id,deps.admitted.channel_id,crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,1,1,1,"runtime-scope-assignment:1",2,pin).map_err(debug)?;
        let reservation=runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
        deps.actor=runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.initialize_pinned_test_position(deps.actor,super::super::quest_world_callbacks::local(start)).map_err(debug)?;
        deps.runtime=tokio::sync::Mutex::new(runtime);
        let mut owner=deps.owner(&h,&authority);
        owner.active_generation=Some(generation);
        owner.movement_cells=&cells;
        owner.quest_catalogue=Some(Arc::clone(&catalogue));
        let mut admitted=deps.admitted;
        admitted.first_entry=crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fence=owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
        let fresh=super::super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root.initialize_admission_equipment(&authority,&h.node,&fence.item_fence(),generation.identity().server_artifact_digest(),fresh.operation()).await.map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        super::super::item_look::seed_main_backpack_definition(&h,&[],"oteryn:item.tibia.i2853").await?;
        let mut selected=admitted.continuity.selected_capabilities.as_slice().to_vec();
        selected.extend([3,4,6,11,15,18,25]); selected.sort_unstable(); selected.dedup();
        let offered=crate::gameplay_transport::appearance_change::offer::offered(owner.offered_capabilities(),owner.appearance_available());
        admitted.continuity.selected_capabilities=crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered,&selected).ok_or("capabilities")?;
        let target_for=|at:Position|->TestResult<WorldObjectTarget>{
            let key=oteryn_world_bundle::bundle::placement_key(at.floor,at.x,at.y,1).ok_or("target")?;
            let mut placement=world.base().digest().to_vec();placement.extend_from_slice(&key.to_be_bytes());
            Ok(WorldObjectTarget{placement,expected_revision:0})
        };
        seed(&owner,&admitted,&prior(1),35000).await?;
        let command=admitted.continuity.next_command_id;
        use_source(&mut owner,&mut admitted,command,target_for(AT)?).await?;
        assert_eq!(quantity(&h,PARTS[3]).await?,0,"wrong stage cannot grant map part four");
        seed(&owner,&admitted,&prior(2),35001).await?;
        let command=admitted.continuity.next_command_id;
        let result=use_source(&mut owner,&mut admitted,command,target_for(AT)?).await?;
        let agaric=result==UseDisposition::Committed && quantity(&h,PARTS[3]).await?==1 && progress(&owner,&admitted,&authority,STONE).await?==3;
        let held=quantity(&h,PARTS[3]).await?;
        assert_eq!(use_source(&mut owner,&mut admitted,command,target_for(AT)?).await?,result);
        let command=admitted.continuity.next_command_id;
        use_source(&mut owner,&mut admitted,command,target_for(AT)?).await?;
        assert_eq!(quantity(&h,PARTS[3]).await?,held,"replay/fresh repeat cannot duplicate fourth part");
        let mut composed=0;
        for (variant,selected) in PARTS.iter().enumerate(){
            // Independent source inputs only, never complete maps or reward receipts.
            for (offset,item) in PARTS.iter().enumerate(){
                let ordinal:i64=sqlx::query_scalar("SELECT (coalesce(max(placement_ordinal),0)+1)::bigint FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(id(70).as_slice()).fetch_one(&h.pool).await?;
                append_fixture_input(&h,item,100+u8::try_from(variant*4+offset)?,i32::try_from(ordinal)?).await?;
            }
            let mut before=Vec::new();for item in PARTS{before.push(quantity(&h,item).await?);}
            let maps_before=quantity(&h,MAP).await?;
            let (result,actual,command)=use_piece(&mut owner,&mut admitted,selected).await?;
            let mut after=Vec::new();for item in PARTS{after.push(quantity(&h,item).await?);}
            let maps_after=quantity(&h,MAP).await?;
            let correct=result==UseDisposition::Committed && before.iter().zip(&after).all(|(a,b)|a-b==1) && maps_after==maps_before+1 && progress(&owner,&admitted,&authority,STONE).await?==4;
            if correct{composed+=1;}
            let (end,replay,_)=super::super::rust_lottery::exchange(&owner,admitted,command,&actual,None,None).await?;
            admitted=end;assert_eq!(replay,result,"literal carried-source replay retains its exact result");
            for (item,expected) in PARTS.iter().zip(after){assert_eq!(quantity(&h,item).await?,expected,"replay cannot consume another fragment");}
            assert_eq!(quantity(&h,MAP).await?,maps_after,"replay cannot duplicate map");
        }
        // This is the independent treasure entry's required prior stage; its
        // stage5, three assets and receipt are never supplied by a fixture.
        seed(&owner,&admitted,&prior(4),35100).await?;
        relocate(&owner,&admitted,neighbor(WRONG)).await?;
        let command=admitted.continuity.next_command_id;
        use_source(&mut owner,&mut admitted,command,target_for(WRONG)?).await?;
        for item in ["oteryn:item.tibia.i25698","oteryn:item.tibia.i25737","oteryn:item.tibia.i24390"]{assert_eq!(quantity(&h,item).await?,0,"same mosaic away from exact source position cannot grant");}
        relocate(&owner,&admitted,neighbor(TREASURE)).await?;
        let command=admitted.continuity.next_command_id;
        let result=use_source(&mut owner,&mut admitted,command,target_for(TREASURE)?).await?;
        assert_eq!(use_source(&mut owner,&mut admitted,command,target_for(TREASURE)?).await?,result);
        let command=admitted.continuity.next_command_id;
        use_source(&mut owner,&mut admitted,command,target_for(TREASURE)?).await?;
        let first_treasure=result==UseDisposition::Committed
            && quantity(&h,"oteryn:item.tibia.i25698").await?==1
            && quantity(&h,"oteryn:item.tibia.i25737").await?==5
            && quantity(&h,"oteryn:item.tibia.i24390").await?==5
            && progress(&owner,&admitted,&authority,STONE).await?==5;
        // The donor composition has no stage gate: another four actual parts
        // can assemble the map again and reset5->4 for a second earned treasure.
        let mut before=Vec::new();
        for (offset,item) in PARTS.iter().enumerate(){
            let ordinal:i64=sqlx::query_scalar("SELECT (coalesce(max(placement_ordinal),0)+1)::bigint FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(id(70).as_slice()).fetch_one(&h.pool).await?;
            append_fixture_input(&h,item,120+u8::try_from(offset)?,i32::try_from(ordinal)?).await?;
            before.push(quantity(&h,item).await?);
        }
        let maps_before=quantity(&h,MAP).await?;
        let (repeat,actual,command)=use_piece(&mut owner,&mut admitted,PARTS[0]).await?;
        let mut consumed=true;for (item,before) in PARTS.iter().zip(before){consumed &= quantity(&h,item).await?==before-1;}
        let repeated_map=repeat==UseDisposition::Committed && consumed && quantity(&h,MAP).await?==maps_before+1 && progress(&owner,&admitted,&authority,STONE).await?==4;
        eprintln!("THREATENED_TREASURE REPEAT command={command} result={repeat:?} consumed={consumed} maps_before={maps_before} maps_after={} stage={}", quantity(&h,MAP).await?, progress(&owner,&admitted,&authority,STONE).await?);
        let (end,replay,_)=super::super::rust_lottery::exchange(&owner,admitted,command,&actual,None,None).await?;
        admitted=end;assert_eq!(replay,repeat);
        let command=admitted.continuity.next_command_id;
        let second=use_source(&mut owner,&mut admitted,command,target_for(TREASURE)?).await?;
        assert_eq!(use_source(&mut owner,&mut admitted,command,target_for(TREASURE)?).await?,second);
        let command=admitted.continuity.next_command_id;
        use_source(&mut owner,&mut admitted,command,target_for(TREASURE)?).await?;
        let ring=quantity(&h,"oteryn:item.tibia.i25698").await?;
        let quartz=quantity(&h,"oteryn:item.tibia.i25737").await?;
        let coins=quantity(&h,"oteryn:item.tibia.i24390").await?;
        let stage=progress(&owner,&admitted,&authority,STONE).await?;
        let treasure=first_treasure && repeated_map && second==UseDisposition::Committed && (ring,quartz,coins,stage)==(2,10,10,5);
        for at in [AT,TREASURE,WRONG]{let key=oteryn_world_bundle::bundle::placement_key(at.floor,at.x,at.y,1).ok_or("map key")?;assert_eq!(owner.bundle_map().ok_or("map")?.facts.object_revision(key),0,"per-player item rewards do not mutate global source objects");}
        let earned=usize::from(agaric)+composed+usize::from(treasure);
        eprintln!("THREATENED_TREASURE POSTGRES_EVIDENCE entries={earned} agaric={agaric} maps={composed} repeated_map={repeated_map} ring={ring} quartz={quartz} coins={coins} stage={stage}");
        drop(owner);drop(authority);drop(seal);h.cleanup().await?;
        assert_eq!((earned,composed,repeated_map,ring,quartz,coins,stage),(6,4,true,2,10,10,5),"all six exact source entries and repeated earned cycle must consume, mint and progress through their real owners");
        Ok(())
 })
}
async fn use_piece(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,key:&str) -> TestResult<(UseDisposition,Vec<u8>,u64)>{
    let native=owner.active_generation.and_then(|g|g.native_gameplay()).ok_or("native")?;
    let source=native.item_index().ok_or("item index")?.definition_ref("Item",key,"definition-r1").ok_or("native fragment reference")?;
    let payload=encode_item_use_intent(&UseIntent{target:UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),use_with:None,use_with_map_item:None,use_with_position:None}).map_err(debug)?;
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
    admitted.item_fence=Some(owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?.item_fence());
    let command=admitted.continuity.next_command_id;
    let (end,result,actual)=super::super::rust_lottery::exchange(owner,*admitted,command,&payload,Some(source),None).await?;
    *admitted=end;Ok((result,actual,command))
}
