//! Save-qualified event history must not inherit global canonical completion.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};

const QUEST: &str = "oteryn:quest.qm55_save_event";
const EMPTY: &str = "oteryn:quest.qm55_completion_only_event";
const TRACK: &str = "oteryn:quest-progress/qm55/save-event";
const BEGIN: &str = "oteryn:quest-transition/qm55/save-event-begin";
const FINISH: &str = "oteryn:quest-transition/qm55/save-event-finish";
const EMPTY_FINISH: &str = "oteryn:quest-transition/qm55/save-event-empty";

#[test]
fn qm55_save_event_admission_retains_latest_values_and_trackless_completion() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "qm55_save_event_progress", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M55_PG_EXECUTED save_event_progress duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M55 save event worker failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let world = f::scope()?.world_id();
    for epoch in 1..=2_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    assert!(
        h.root
            .world_changes(world, 1)
            .await?
            .ok_or("current day")?
            .grimvale_full_moon
    );
    let transition = |key: &str, quest: &str, effects, completes| QuestTransition {
        key: key.into(),
        quest: quest.into(),
        effects,
        completes,
        experience: None,
        world_change: Some("grimvale".into()),
    };
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: TRACK.into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: 2,
            }],
            vec![
                transition(
                    BEGIN,
                    QUEST,
                    vec![QuestEffect {
                        track: TRACK.into(),
                        from: QuestComparison::Between(0, 0),
                        effect: QuestEffectKind::Set(1),
                    }],
                    false,
                ),
                transition(
                    FINISH,
                    QUEST,
                    vec![QuestEffect {
                        track: TRACK.into(),
                        from: QuestComparison::Between(1, 1),
                        effect: QuestEffectKind::Set(2),
                    }],
                    true,
                ),
                transition(EMPTY_FINISH, EMPTY, vec![], true),
            ],
        )
        .map_err(f::debug)?,
    );
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let character = f::fence(1)?.character_id;
    let request = |key: &str, ordinal| -> f::TestResult<_> {
        Ok(QuestTransitionRequest {
            transition_key: key.into(),
            cause: QuestCause::Command(CommandRef::new(
                GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
                CommandId::new(ordinal).map_err(f::debug)?,
            )),
        })
    };
    for (key, revision, ordinal) in [(BEGIN, 1, 1), (FINISH, 2, 2), (EMPTY_FINISH, 3, 3)] {
        let result = h
            .root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(revision)?,
                request(key, ordinal)?,
                catalogue.clone(),
            )
            .await?;
        assert!(
            matches!(&result, QuestTransitionOutcome::Committed(_)),
            "{key}: {result:?}"
        );
        let copy = h
            .root
            .read_character_quest_state(&authority, character)
            .await?;
        let progress = copy
            .save_event_quest_progress(QUEST)
            .ok_or("current save missing event progress")?;
        assert_eq!(progress.reset_epoch, 1);
        assert_eq!(progress.completed, key != BEGIN);
        assert_eq!(
            progress.tracks.get(TRACK),
            Some(&if key == BEGIN { 1 } else { 2 })
        );
        if let QuestTransitionOutcome::Committed(receipt) = &result {
            let mut delta = copy.clone();
            assert!(delta.apply(receipt));
            assert!(delta.save_event_quest_progress(QUEST).is_none());
        }
    }
    let copy = h
        .root
        .read_character_quest_state(&authority, character)
        .await?;
    let empty = copy
        .save_event_quest_progress(EMPTY)
        .ok_or("zero-track completion was dropped")?;
    assert!(empty.completed);
    assert_eq!(empty.reset_epoch, 1);
    assert!(empty.tracks.is_empty());
    let duplicate = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(4)?,
            request(EMPTY_FINISH, 4)?,
            catalogue.clone(),
        )
        .await?;
    assert!(
        matches!(duplicate, QuestTransitionOutcome::Refused(_)),
        "fresh command cannot complete twice in a save: {duplicate:?}"
    );
    assert_eq!(h.root_revision().await?, "4");
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    h.root.world_changes(world, 2).await?.ok_or("next day")?;
    let copy = h
        .root
        .read_character_quest_state(&authority, character)
        .await?;
    for quest in [QUEST, EMPTY] {
        assert!(copy.save_event_quest_progress(quest).is_none());
        assert!(
            copy.states()
                .get(quest)
                .ok_or("canonical history")?
                .completed
        );
    }
    let next_completion = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(4)?,
            request(EMPTY_FINISH, 5)?,
            catalogue.clone(),
        )
        .await?;
    assert!(
        matches!(next_completion, QuestTransitionOutcome::Committed(_)),
        "next save permits completion despite canonical history: {next_completion:?}"
    );
    let copy = h
        .root
        .read_character_quest_state(&authority, character)
        .await?;
    let next = copy
        .save_event_quest_progress(EMPTY)
        .ok_or("next save completion missing")?;
    assert_eq!(next.reset_epoch, 2);
    assert!(next.completed);
    assert!(next.tracks.is_empty());
    let duplicate = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(5)?,
            request(EMPTY_FINISH, 6)?,
            catalogue.clone(),
        )
        .await?;
    assert!(matches!(duplicate, QuestTransitionOutcome::Refused(_)));
    assert_eq!(h.root_revision().await?, "5");
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(1)?,
                request(BEGIN, 1)?,
                catalogue
            )
            .await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert!(
        h.root
            .read_character_quest_state(&authority, character)
            .await?
            .save_event_quest_progress(QUEST)
            .is_none(),
        "old receipt cannot bind to a new save"
    );
    Ok(())
}
