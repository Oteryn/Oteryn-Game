//! M47 drives the fenced world writer, preserving the command receipt on replay.
use super::quest_world_callbacks::scenario;
use super::fixture::{Harness,debug};
use super::TestResult;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback,Position};
use crate::durability::character_progression::{CharacterProgressionError, CurrentCharacterGameplayFence};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{barbarian_mead as mead, world_callbacks, QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};
use std::sync::Arc;

#[test]
fn m47_postgres_mead_roll_is_fenced_and_exactly_replayed() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("m47-mead", || {
        scenario("barbarian-test-mead", Position {x:32201,y:31154,floor:-7}, false)
    });
    println!("M47_POSTGRES_EXECUTED duration={:?} success={}",started.elapsed(),result.is_ok());
    assert!(started.elapsed().as_secs_f64() > 0.01);
    result
}

pub(super) async fn exercise(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
) -> TestResult {
    let base = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let tracks = mead::TRACKS.into_iter().map(|k| base.track(k).cloned().ok_or("mead track")).collect::<Result<Vec<_>,_>>()?;
    let setup = QuestTransition {
        key:"oteryn:quest-transition/m47-fixture-round".into(),quest:mead::QUEST.into(),
        effects:mead::TRACKS.into_iter().zip([2,2,0,0]).map(|(track,value)| QuestEffect {track:track.into(),from:QuestComparison::Between(-1,i64::MAX),effect:QuestEffectKind::Set(value)}).collect(),
        completes:false,experience:None,world_change:None,
    };
    let catalogue = Arc::new(QuestStateCatalogue::new(base.content_revision(),tracks,vec![setup.clone(),world_callbacks::primary_transition(callback)?]).map_err(debug)?);
    let request = |key: &str,n| -> TestResult<QuestTransitionRequest> { Ok(QuestTransitionRequest {transition_key:key.into(),cause:QuestCause::Use(CommandRef::new(admitted.game_session_id,CommandId::new(n).map_err(debug)?))}) };
    let (context,permit) = {
        let runtime = owner.runtime.lock().await;
        let actor = admitted.runtime_actor.ok_or("actor")?;
        let context = QuestWorldContext {
            map_revision:crate::map::overlay::map_revision(owner.bundle_map().ok_or("map")?.overlay.base()),
            content_revision:format!("sha256:{}",runtime.content_pin().server_artifact_digest().iter().map(|b|format!("{b:02x}")).collect::<String>()),
            placement_context:runtime.content_pin().frame_binding_digest().to_vec(),reset_epoch:owner.reset_epoch,
            actor_placement:actor.placement_identity(),movement_revision:runtime.read_actor_position(actor).map_err(debug)?.revision(),
        };
        drop(runtime);
        (context,owner.spell_lane_permit().await.ok_or("permit")?)
    };
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let mut start = request(&setup.key,7000)?;
    start.cause = QuestCause::Command(CommandRef::new(admitted.game_session_id,CommandId::new(7000).map_err(debug)?));
    assert!(matches!(slot.commit_quest_transition(owner.root,owner.character,owner.holder,fence,start,catalogue.clone()).await?,QuestTransitionOutcome::Committed(_)));
    let command = request(mead::KEY,7001)?;
    let generic = slot.commit_quest_transition(owner.root,owner.character,owner.holder,fence,command.clone(),catalogue.clone()).await;
    assert!(matches!(generic,Err(CharacterProgressionError::InvalidInput)),"{generic:?}");
    let mut stale = fence;
    stale.character_lease_generation += 1;
    let rejected = slot.commit_world_callback(owner.root,owner.character,owner.holder,stale,command.clone(),catalogue.clone(),&permit,context.clone(),None).await;
    assert!(matches!(rejected,Err(CharacterProgressionError::AuthorityRejected)),"{rejected:?}");
    let mut wrong_world = context.clone();
    wrong_world.reset_epoch += 1;
    let rejected = slot.commit_world_callback(owner.root,owner.character,owner.holder,fence,command.clone(),catalogue.clone(),&permit,wrong_world,None).await?;
    assert!(matches!(rejected,QuestTransitionOutcome::Refused(_)),"{rejected:?}");
    let first = slot.commit_world_callback(owner.root,owner.character,owner.holder,fence,command.clone(),catalogue.clone(),&permit,context.clone(),None).await?;
    let QuestTransitionOutcome::Committed(receipt) = first else { return Err(format!("M47 did not commit: {first:?}").into()); };
    assert_eq!(receipt.changes.len(),4);
    assert_eq!(receipt.changes.iter().find(|c| c.track == mead::TRACKS[3]).ok_or("total")?.after,1);
    for _ in 0..3 {
        let replay = slot.commit_world_callback(owner.root,owner.character,owner.holder,fence,command.clone(),catalogue.clone(),&permit,context.clone(),None).await?;
        assert_eq!(replay,QuestTransitionOutcome::AlreadyCommitted(receipt.clone()));
    }
    let count:i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1").bind(mead::KEY).fetch_one(&h.pool).await?;
    assert_eq!(count,1);
    let stored: Vec<(String,i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE track_key=ANY($1::text[]) ORDER BY track_key").bind(mead::TRACKS.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(stored.len(),4);
    for (track,value) in stored {
        assert_eq!(receipt.changes.iter().find(|c| c.track==track).ok_or("stored receipt track")?.after,value);
    }
    Ok(())
}
