//! M18: committed personal trial progress gates the existing timed Boss Activity owner.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::activity_handoff::fixture_first_dragon_trials_permit;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition};
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
const TRIALS: &str = "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s4";
const DEFEAT: &str = "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s5";

#[test]
fn m18_first_dragon_trial_entry_guard_postgres_committed_progress_and_replay() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else { return Ok(()); };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(),String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = fixture::Harness::create(admin, "season_first_dragon_entry", true).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("M18_PG_EXECUTED first_dragon_entry_guard duration={:?}", started.elapsed());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_| "M18 PostgreSQL thread failed")?.map_err(Into::into)
}
async fn permitted(h: &fixture::Harness) -> fixture::TestResult<bool> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&mut *tx).await?;
    let answer = fixture_first_dragon_trials_permit(&mut tx, fixture::fence(1)?.character_id).await?;
    tx.rollback().await?;
    Ok(answer)
}
async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let effect = |track: &str, from, kind| QuestEffect { track:track.into(), from, effect:kind };
    let transition = |name:&str,effects| QuestTransition {
        key:format!("oteryn:quest-transition/m18-proof/{name}"), quest:QUEST.into(), effects,
        completes:false, experience:None, world_change:None,
    };
    let catalogue = Arc::new(QuestStateCatalogue::new("content-1",vec![
        QuestTrack { key:TRIALS.into(), quest:QUEST.into(), initial:0,min:0,max:2 },
        QuestTrack { key:DEFEAT.into(), quest:QUEST.into(), initial:0,min:0,max:1 },
    ],vec![
        transition("one",vec![effect(TRIALS,QuestComparison::Eq(0),QuestEffectKind::Set(1))]),
        transition("two",vec![effect(TRIALS,QuestComparison::Eq(1),QuestEffectKind::Set(2))]),
        transition("defeat",vec![effect(TRIALS,QuestComparison::Eq(2),QuestEffectKind::Add(0)),effect(DEFEAT,QuestComparison::Eq(0),QuestEffectKind::Set(1))]),
    ]).map_err(fixture::debug)?);
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(fixture::debug)?;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?;
    let request = |name:&str,id| -> fixture::TestResult<QuestTransitionRequest> { Ok(QuestTransitionRequest {
        transition_key:format!("oteryn:quest-transition/m18-proof/{name}"),
        cause:QuestCause::Command(CommandRef::new(session,CommandId::new(id).map_err(fixture::debug)?)),
    }) };
    assert!(!permitted(h).await?, "missing progress must refuse entry");
    for (name,revision,id,expected) in [("one",1,22101,false),("two",2,22102,true),("defeat",3,22103,false)] {
        let outcome = h.root.commit_character_quest_transition(&authority,&h.node,fixture::fence(revision)?,request(name,id)?,catalogue.clone()).await?;
        assert!(matches!(outcome,QuestTransitionOutcome::Committed(_)),"{outcome:?}");
        assert_eq!(permitted(h).await?,expected,"{name}");
        let replay = h.root.commit_character_quest_transition(&authority,&h.node,fixture::fence(revision)?,request(name,id)?,catalogue.clone()).await?;
        assert!(matches!(replay,QuestTransitionOutcome::AlreadyCommitted(_)));
        assert_eq!(permitted(h).await?,expected,"replay of {name}");
    }
    assert_eq!(h.root_revision().await?,"4");
    Ok(())
}
