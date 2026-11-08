//! Five actual fairy AIDs share durable once flags/counter and the Channel timed transform owner.
use super::super::fixture::{Harness,debug,id,run};
use super::super::{Dependencies,TestResult,prepare_current_session};
use super::barriers::{progress,relocate};
use super::rakes::seed;
use super::acquisition::use_source;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Passage,Position};
use oteryn_gameplay_quests::quest::{QuestComparison,QuestEffect,QuestEffectKind,QuestStateCatalogue,QuestTrack,QuestTransition};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition,WorldObjectTarget};
use std::sync::Arc;
use oteryn_gameplay_domain::domain::appearance::AppearanceContent;
const QUEST:&str="oteryn:quest.threatened_dreams_quest";
const NIGHT:&str="oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders";
const COUNTER:&str="oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/fairies_counter";
const PREFIX:&str="oteryn:quest-transition/questpc3/threatened-fairy/";
const TARGETS:[Position;5]=[Position{x:33576,y:32185,floor:-8},Position{x:33621,y:32214,floor:-8},Position{x:33559,y:32203,floor:-9},Position{x:33505,y:32286,floor:-8},Position{x:33440,y:32217,floor:-8}];
fn flag(i:usize)->String{format!("oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/fairy{:02}",i+1)}
fn prior(value:i64)->String{format!("oteryn:quest-transition/fixture/fairies/prior-night{value}")}
fn neighbor(at:Position)->Position{Position{x:at.x-1,..at}}
fn catalogue()->TestResult<QuestStateCatalogue>{
 let source=oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
 let mut tracks=vec![source.catalogue().track(NIGHT).ok_or("night source")?.clone(),source.catalogue().track(COUNTER).ok_or("counter source")?.clone()];
 for i in 0..5{tracks.push(source.catalogue().track(&flag(i)).cloned().unwrap_or(QuestTrack{key:flag(i),quest:QUEST.into(),initial:-1,min:-1,max:1}));}
 let mut transitions=Vec::new();
 for c in oteryn_gameplay_quests::quest::world_callbacks::load()?.into_iter().filter(|c|c.key.starts_with(PREFIX)){transitions.push(source.catalogue().transition(&c.key).ok_or("source fairy transition")?.clone());}
 for value in [2,3]{transitions.push(QuestTransition{
world_change: None,key:prior(value),quest:QUEST.into(),effects:vec![QuestEffect{track:NIGHT.into(),from:QuestComparison::Any,effect:QuestEffectKind::Set(value)}],completes:false,experience:None});}
 QuestStateCatalogue::new("content-1",tracks,transitions).map_err(|e|format!("{e:?}").into())
}
#[test]
fn threatened_five_fairies_are_distinct_capped_and_restore_in_idle_channel()->TestResult{
 if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none(){eprintln!("THREATENED_FAIRIES POSTGRES_SKIPPED no_admin_url");return Ok(());}
 std::thread::Builder::new().stack_size(256<<20).spawn(||{let started=std::time::Instant::now();match std::panic::catch_unwind(scenario){Ok(result)=>{eprintln!("THREATENED_FAIRIES POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());result.map_err(|e|e.to_string())},Err(panic)=>{eprintln!("THREATENED_FAIRIES POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());std::panic::resume_unwind(panic)}}})?.join().map_err(|_|"fairy scenario panicked")??;Ok(())
}
fn scenario()->TestResult{
 let catalogue=Arc::new(catalogue()?);
 run(async |admin|{
        let h=Harness::create_with_admission_seed(admin,"threatened_fairies",false,false).await?;
        prepare_current_session(&h).await?;
        let version:String=sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&h.pool).await?;
        eprintln!("THREATENED_FAIRIES POSTGRES_CASE_STARTED server_version={version}");
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(|error|format!("fairy authority admission: {error:?}"))?;
        let mut deps=Dependencies::new()?;
        let (controller,activated)=super::super::death_ml_loss_composed::production_generation_with_pin(deps.admitted.world_id,"content-1")?;
        let generation=controller.active().ok_or("generation")?;
        let mut geometry=oteryn_gameplay_quests::quest::world_callbacks::load()?.into_iter().next().ok_or("geometry prototype")?;
        geometry.key="oteryn:quest-transition/fixture/fairies/layout".into();
        geometry.selector.position=Some(TARGETS[0]);
        geometry.selector.action=Some(40016); geometry.selector.unique=None;geometry.selector.appearance=Some(25796);
        geometry.lever=None;geometry.teleport=None;geometry.item_on_target=None;geometry.carried_use=None;geometry.offerings.clear();geometry.effects=Default::default();
        geometry.passages=vec![Passage{position:Position{y:TARGETS[0].y+1,..TARGETS[0]},appearance:25797,replacement:None}];
        let additional=TARGETS.iter().enumerate().skip(1).map(|(i,at)|{let mut c=geometry.clone();c.selector.position=Some(*at);c.selector.action=Some(40016+u16::try_from(i).unwrap_or(0));c.passages.clear();c}).collect::<Vec<_>>();
        let world=super::super::quest_world_callbacks::world_with_triggers(&geometry,TARGETS[0],&additional)?;
        let cells=world.movement_cells(activated.qualified_room().movement_cells()).map_err(debug)?;
        let start=neighbor(TARGETS[0]);
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
        super::super::maintain_ready(&h.root).await?;
        let fence=owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
        let fresh=super::super::fresh_fixture::fresh::Source::new(0)?.request()?;
        super::super::maintain_ready(&h.root).await?;
        h.root.initialize_admission_equipment(&authority,&h.node,&fence.item_fence(),generation.identity().server_artifact_digest(),fresh.operation()).await.map_err(|error|format!("fairy admission equipment: {error:?}"))?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        super::super::item_look::seed_main_backpack(&h, &[]).await?;
        // The older admission fixture predates native appearance creation.
        // Seed creation sex as the existing appearance fixtures do, not quest results.
        let mut creation = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *creation).await?;
        sqlx::query("UPDATE game_character_roots SET sex=1 WHERE character_id=encode($1,'hex')::uuid AND sex IS NULL").bind(id(41).as_slice()).execute(&mut *creation).await?;
        creation.commit().await?;
        let mut selected=admitted.continuity.selected_capabilities.as_slice().to_vec();selected.extend([3,4,6,11,15,18,25]);selected.sort_unstable();selected.dedup();
        let offered=crate::gameplay_transport::appearance_change::offer::offered(owner.offered_capabilities(),owner.appearance_available());
        admitted.continuity.selected_capabilities=crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered,&selected).ok_or("capabilities")?;
        let target_for=|at:Position|->TestResult<WorldObjectTarget>{
            let key=oteryn_world_bundle::bundle::placement_key(at.floor,at.x,at.y,1).ok_or("target")?;
            let mut placement=world.base().digest().to_vec();placement.extend_from_slice(&key.to_be_bytes());
            Ok(WorldObjectTarget{placement,expected_revision:world.facts().object_revision(key)})
        };
        let appearance=|at:Position|->TestResult<u16>{let pos=crate::map::overlay::TilePos{x:at.x,y:at.y,floor:at.floor};Ok(world.facts().base_entry(pos,1,1).ok_or("fairy appearance")?.appearance_id)};
        seed(&owner,&admitted,&prior(2),36000).await.map_err(|error|format!("fairy prerequisite: {error}"))?;
        let appearance_fence = crate::gameplay_transport::character_gameplay_fence::current(&owner, admitted.game_session_id, false).await.ok_or("appearance fence")?;
        let (_, cosmetics) = owner.appearance_context().ok_or("appearance content")?;
        h.root.read_current_character_appearance_state(&authority, &h.node, appearance_fence, cosmetics.earned_grant_keys()).await.map_err(|error|format!("fairy appearance read: {error:?}"))?;
        let command=admitted.continuity.next_command_id;
        use_source(&mut owner,&mut admitted,command,target_for(TARGETS[0])?).await?;
        assert_ne!(progress(&owner,&admitted,&authority,&flag(0)).await?,1,"wrong mission stage cannot release a fairy");
        assert_eq!(progress(&owner,&admitted,&authority,COUNTER).await?,0);
        assert_eq!(appearance(TARGETS[0])?,25796);
        seed(&owner,&admitted,&prior(3),36001).await?;
        let mut released=0;let mut scheduled=0;
        for (i,at) in TARGETS.iter().enumerate(){
            relocate(&owner,&admitted,neighbor(*at)).await?;
            let before=progress(&owner,&admitted,&authority,COUNTER).await?;
            let command=admitted.continuity.next_command_id;
            let target=target_for(*at)?;
            let result=use_source(&mut owner,&mut admitted,command,target.clone()).await?;
            let count=progress(&owner,&admitted,&authority,COUNTER).await?;
            let one=progress(&owner,&admitted,&authority,&flag(i)).await?;
            if result==UseDisposition::Committed && one==1 && count==before+1 && appearance(*at)?==25797{released+=1;}
            if world.facts().next_movement_decay_time().is_some(){scheduled+=1;}
            assert_eq!(use_source(&mut owner,&mut admitted,command,target).await?,result,"literal replay retains its receipt despite the transformed target");
            assert_eq!(progress(&owner,&admitted,&authority,COUNTER).await?,count,"replay cannot count twice");
            if i==0{let command=admitted.continuity.next_command_id;use_source(&mut owner,&mut admitted,command,target_for(*at)?).await?;assert_eq!(progress(&owner,&admitted,&authority,COUNTER).await?,count,"fresh repeat cannot count the same fairy twice");}
        }
        let counter=progress(&owner,&admitted,&authority,COUNTER).await?;
        let mut flags=0;for i in 0..5{flags+=usize::from(progress(&owner,&admitted,&authority,&flag(i)).await?==1);}
        let mut restored=0;
        eprintln!("THREATENED_FAIRIES BEFORE_TIMER entries={released} flags={flags} counter={counter} scheduled={scheduled}");
        if let Some(due)=world.facts().next_movement_decay_time(){
            assert_eq!(world.facts().expire_movement_items(due.saturating_sub(1)),0,"the earliest timer must not restore before its exact deadline");
            owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(31)).ok_or("clock")?;
            let stop=oteryn_foundation::CancellationToken::new();
            let cycles=owner.run_source_owner_cycles(&stop);
            let stopping=async{tokio::time::sleep(std::time::Duration::from_millis(150)).await;stop.cancel();};
            use std::future::{Future,poll_fn};
            let mut cycles=std::pin::pin!(cycles);let mut stopping=std::pin::pin!(stopping);let mut stopped=false;
            tokio::time::timeout(std::time::Duration::from_secs(5),poll_fn(|cx|{if !stopped && stopping.as_mut().poll(cx).is_ready(){stopped=true;}cycles.as_mut().poll(cx)})).await?;
            for at in TARGETS{let key=oteryn_world_bundle::bundle::placement_key(at.floor,at.x,at.y,1).ok_or("key")?;restored+=usize::from(appearance(at)?==25796 && world.facts().object_revision(key)>=2);}
        }
        eprintln!("THREATENED_FAIRIES POSTGRES_EVIDENCE entries={released} flags={flags} counter={counter} scheduled={scheduled} restored={restored}");
        drop(owner);drop(authority);drop(seal);h.cleanup().await?;
        assert_eq!((released,flags,counter,scheduled,restored),(5,5,5,5,5),"five actual AIDs must each advance once and restore through the idle Channel timer owner");
        Ok(())
 })
}
