//! SRV6: the source's post-battle Planestrider conversation, not a raw kill, awards Planegazer.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, WorldId};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use std::sync::Arc;
const NPC: &str = "oteryn:npc.planestrider_npc";
const QUEST: &str = "oteryn:quest.opticording_sphere_quest";
const READY: &str = "oteryn:quest-progress/opticording_sphere/planestrider_defeated";
const CLAIMED: &str = "oteryn:quest-progress/opticording_sphere/final_report";
const COMPLETE: &str = "oteryn:quest-transition/opticording-sphere/planegazer-final-report";

#[test]
fn srv6_planegazer_real_npc_exact_finale_sequence_and_title() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(|| {
        run(async |admin| {
            let began=std::time::Instant::now();
            let h=Harness::create_with_admission_seed(admin,"srv6_planegazer",true,false).await?;
            prepare_current_session(&h).await?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
            let world=WorldId::decode(&id(42)).map_err(debug)?;
            let (generation,pin)=super::death_ml_loss_composed::production_generation_with_pin(world,"content-1")?;
            let active=generation.active().ok_or("active")?;
            let room=pin.qualified_room().clone();
            let (channel_pin,_,_,_)=pin.into_channel_parts();
            let deps=Dependencies::with_content(channel_pin,room,active.native_gameplay().ok_or("native")?.spell_book().clone())?;
            deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
            let mut owner=deps.owner(&h,&authority);
            owner.active_generation=Some(active);
            let canonical=oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?.catalogue().clone();
            let mut transitions=canonical.transition(COMPLETE).cloned().into_iter().collect::<Vec<_>>();
            transitions.push(QuestTransition{world_change:None,key:"oteryn:quest-transition/srv6-planegazer-ready".into(),quest:QUEST.into(),effects:vec![QuestEffect{track:READY.into(),from:QuestComparison::Any,effect:QuestEffectKind::Set(1)}],completes:false,experience:None});
            let tracks=[READY,CLAIMED].into_iter().map(|key|QuestTrack{key:key.into(),quest:QUEST.into(),initial:0,min:0,max:1}).collect();
            owner.quest_catalogue=Some(Arc::new(QuestStateCatalogue::new("content-1",tracks,transitions).map_err(debug)?));
            deps.initialize(&owner).await;
            let mut admitted=deps.admitted;
            admitted.first_entry=FirstEntryOutcome::Positioned;
            let fence=owner.current_quest_fence(admitted.game_session_id).await.map_err(|_|"fence")?.ok_or("fence")?;
            admitted.item_fence=Some(fence.item_fence());
            owner.admit_quest_session(&admitted).await;
            let [npc]=install(&owner,&admitted,[NPC]).await?;
            talk(&mut owner,&admitted,npc,1000,"hi").await?;
            let before:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=91").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(before,0);
            let mut slot=owner.revision_sequencer.acquire(fence.character_id).await;
            let result=slot.commit_quest_transition(owner.root,owner.character,owner.holder,fence,QuestTransitionRequest{transition_key:"oteryn:quest-transition/srv6-planegazer-ready".into(),cause:QuestCause::Command(CommandRef::new(admitted.game_session_id,CommandId::new(2000).map_err(debug)?))},Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?)).await.map_err(debug)?;
            assert!(matches!(result,QuestTransitionOutcome::Committed(_)));drop(slot);
            owner.refresh_quest_session(admitted.game_session_id,false).await;
            for (i,word) in ["hi","no","yes","no","yes","yes","yes"].into_iter().enumerate() {
                let lines=talk(&mut owner,&admitted,npc,2010+2*u64::try_from(i)?,word).await?;
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=91").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(count,i64::from(i==6),"only exact final dialogue grants title");
                if i==6 {assert!(lines.iter().any(|line|line.contains("Planegazer")));}
                if i==0 {
                    let _=talk(&mut owner,&admitted,npc,2011,"yes").await?;
                    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=91").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                    assert_eq!(count,0,"wrong branch cannot skip the source sequence");
                }
            }
            let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
            assert!(completed);
            eprintln!("SRV6_PLANEGAZER_NPC_POSTGRES_COMPLETED elapsed={:?}",began.elapsed());
            drop(owner);drop(authority);drop(seal);h.cleanup().await
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"Planegazer NPC scenario panicked")?.map_err(Into::into)
}
