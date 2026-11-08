//! Only prior Animals6/7 and an empty backpack are fixtures; notes require actual native world USE.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session};
use super::barriers::{progress, quantity, relocate};
use super::rakes::seed;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Passage, Position};
use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

const ANIMALS: &str = oteryn_gameplay_quests::quest::threatened_dreams::ANIMALS;
const NOTES: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission01/poacher_notes";
const ITEM: &str = "oteryn:item.tibia.i25242";
const KEY: &str = "oteryn:quest-transition/questpc3/threatened-acquisition/poacher-notes";
const PRIOR6: &str = "oteryn:quest-transition/fixture/threatened-acquisition/animals6";
const PRIOR7: &str = "oteryn:quest-transition/fixture/threatened-acquisition/animals7";
// Fixture coordinates only: source registers globally by AID20002, without a position/appearance gate.
const AT: Position = Position { x: 32950, y: 31810, floor: -7 };
const WRONG: Position = Position { x: 32954, y: 31810, floor: -7 };
fn neighbor(at: Position) -> Position { Position { x: at.x-1, ..at } }

#[test]
fn threatened_acquisition_poacher_notes_requires_exact_source_and_preserves_animals() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_POACHER_NOTES POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(|| {
        let started=std::time::Instant::now();
        match std::panic::catch_unwind(scenario) {
            Ok(result) => { eprintln!("THREATENED_POACHER_NOTES POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok()); result.map_err(|e|e.to_string()) },
            Err(panic) => { eprintln!("THREATENED_POACHER_NOTES POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis()); std::panic::resume_unwind(panic) }
        }
    })?.join().map_err(|_|"notes scenario panicked")??;
    Ok(())
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let source=oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
    let mut tracks=Vec::new();
    for key in [ANIMALS,NOTES] { tracks.push(source.catalogue().track(key).ok_or("source track")?.clone()); }
    let mut transitions=Vec::new();
    if let Some(callback)=source.catalogue().transition(KEY) { transitions.push(callback.clone()); }
    for (key,value) in [(PRIOR6,6),(PRIOR7,7)] {
        transitions.push(QuestTransition {
world_change: None, key:key.into(), quest:"oteryn:quest.threatened_dreams_quest".into(), effects:vec![QuestEffect {track:ANIMALS.into(),from:QuestComparison::Any,effect:QuestEffectKind::Set(value)}], completes:false,experience:None });
    }
    QuestStateCatalogue::new("content-1",tracks,transitions).map_err(|e|format!("{e:?}").into())
}

fn scenario() -> TestResult {
    let catalogue=Arc::new(catalogue()?);
    run(async |admin| {
        let h=Harness::create_with_admission_seed(admin,"threatened_notes",false,false).await?;
        prepare_current_session(&h).await?;
        let version:String=sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&h.pool).await?;
        eprintln!("THREATENED_POACHER_NOTES POSTGRES_CASE_STARTED server_version={version}");
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut deps=Dependencies::new()?;
        let (controller,activated)=super::super::death_ml_loss_composed::production_generation_with_pin(deps.admitted.world_id,"content-1")?;
        let generation=controller.active().ok_or("generation")?;
        let mut geometry=oteryn_gameplay_quests::quest::world_callbacks::load()?.into_iter().next().ok_or("geometry prototype")?;
        geometry.selector.position=Some(AT);
        geometry.selector.action=Some(20002);
        geometry.selector.unique=None;
        geometry.selector.appearance=Some(100);
        geometry.lever=None;
        geometry.teleport=None;
        geometry.item_on_target=None;
        geometry.carried_use=None;
        geometry.offerings.clear();
        geometry.passages=vec![Passage { position:WRONG,appearance:100,create_on_close:None,replacement:None }];
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
        super::super::item_look::seed_main_backpack(&h,&[]).await?;
        let mut selected=admitted.continuity.selected_capabilities.as_slice().to_vec();
        selected.extend([3,6]); selected.sort_unstable(); selected.dedup();
        let offered=crate::gameplay_transport::appearance_change::offer::offered(owner.offered_capabilities(),owner.appearance_available());
        admitted.continuity.selected_capabilities=crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered,&selected).ok_or("capabilities")?;
        let target_for=|at:Position| -> TestResult<WorldObjectTarget> {
            let key=oteryn_world_bundle::bundle::placement_key(at.floor,at.x,at.y,1).ok_or("target")?;
            let mut placement=world.base().digest().to_vec(); placement.extend_from_slice(&key.to_be_bytes());
            Ok(WorldObjectTarget { placement,expected_revision:0 })
        };
        seed(&owner,&admitted,PRIOR6,34000).await?;
        use_source(&mut owner,&mut admitted,1,target_for(AT)?).await?;
        assert_eq!(quantity(&h,ITEM).await?,0,"wrong Animals phase cannot grant notes");
        assert_eq!(progress(&owner,&admitted,&authority,NOTES).await?,0);
        seed(&owner,&admitted,PRIOR7,34001).await?;
        let fence=owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
        let mut slot=owner.revision_sequencer.acquire(fence.character_id).await;
        let weaker=slot.commit_quest_transition(owner.root,owner.character,owner.holder,fence,
            QuestTransitionRequest { transition_key:KEY.into(),cause:QuestCause::Command(CommandRef::new(admitted.game_session_id,CommandId::new(34002).map_err(debug)?)) },
            Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?)).await;
        assert!(matches!(weaker,Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)),"world value cannot be granted by a context-free quest caller");
        drop(slot);
        assert_eq!(quantity(&h,ITEM).await?,0);
        assert_eq!(progress(&owner,&admitted,&authority,NOTES).await?,0);
        relocate(&owner,&admitted,neighbor(WRONG)).await?;
        use_source(&mut owner,&mut admitted,2,target_for(WRONG)?).await?;
        assert_eq!(quantity(&h,ITEM).await?,0,"matching appearance without source AID cannot grant");
        assert_eq!(progress(&owner,&admitted,&authority,NOTES).await?,0);
        relocate(&owner,&admitted,neighbor(AT)).await?;
        let result=use_source(&mut owner,&mut admitted,3,target_for(AT)?).await?;
        assert_eq!(use_source(&mut owner,&mut admitted,3,target_for(AT)?).await?,result,"exact replay retains original result");
        use_source(&mut owner,&mut admitted,4,target_for(AT)?).await?;
        let physical=quantity(&h,ITEM).await?;
        let flag=progress(&owner,&admitted,&authority,NOTES).await?;
        let animals=progress(&owner,&admitted,&authority,ANIMALS).await?;
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1").bind(KEY).fetch_one(&h.pool).await?;
        let key=oteryn_world_bundle::bundle::placement_key(AT.floor,AT.x,AT.y,1).ok_or("source key")?;
        assert_eq!(owner.bundle_map().ok_or("map")?.facts.object_revision(key),0,"per-player acquisition leaves global source intact");
        eprintln!("THREATENED_POACHER_NOTES POSTGRES_EVIDENCE result={result:?} physical={physical} flag={flag} animals={animals} receipts={receipts}");
        drop(owner); drop(authority); drop(seal); h.cleanup().await?;
        assert_eq!((result,physical,flag,animals,receipts),(UseDisposition::Committed,1,1,7,1),"physical notes and source flag must share the exact native world reward transaction");
        Ok(())
    })
}

pub(super) async fn use_source(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,command:u64,target:WorldObjectTarget)->TestResult<UseDisposition> {
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
    admitted.item_fence=Some(owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?.item_fence());
    let payload=oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let result=super::super::quest_world_callbacks::wire(owner,admitted,command,2,payload).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(|e|format!("{e:?}").into())
}
