//! Personal daily recipes use the save boundary, without activating a global World event.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{
    QuestCause, QuestTransitionOutcome as O, QuestTransitionRequest,
};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison as C, QuestEffect, QuestEffectKind as E, QuestStateCatalogue, QuestTrack,
    QuestTransition, shared_world_progress,
};

#[test]
fn qm55_personal_daily_scope_uses_current_save_without_global_event_activation() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "qm55_personal_daily", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M55_PG_EXECUTED personal_daily duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "personal daily worker failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    const QUEST: &str = "oteryn:quest.qm55_personal_daily";
    const TRACK: &str = "oteryn:quest-progress/qm55/personal-daily";
    const COMPLETE: &str = "oteryn:quest-transition/qm55/personal-daily-complete";
    let world = f::scope()?.world_id();
    for epoch in [1_i64, 2] {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-12T12:00:00Z','2031-06')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    }
    let reset = shared_world_progress::save_event_restart_key(QUEST);
    let make = |key: &str, from, value, completes| QuestTransition {
        key: key.into(),
        quest: QUEST.into(),
        effects: vec![QuestEffect {
            track: TRACK.into(),
            from,
            effect: E::Set(value),
        }],
        completes,
        experience: None,
        world_change: Some("server_save".into()),
    };
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: TRACK.into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![
                make(&reset, C::Any, 0, false),
                make(COMPLETE, C::Eq(0), 1, true),
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
    let request = |key: &str, ordinal| -> f::TestResult<_> {
        Ok(QuestTransitionRequest {
            transition_key: key.into(),
            cause: QuestCause::Command(CommandRef::new(
                GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
                CommandId::new(ordinal).map_err(f::debug)?,
            )),
        })
    };
    let missing = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(1)?,
            request(&reset, 10)?,
            catalogue.clone(),
        )
        .await?;
    assert!(
        matches!(missing, O::Refused(_)),
        "no published save must refuse admission"
    );
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let bare_quest = "oteryn:quest.qm55_bare_personal_scope";
    let bare_key = "oteryn:quest-transition/qm55/bare-personal-scope";
    let bare = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![],
            vec![QuestTransition {
                key: bare_key.into(),
                quest: bare_quest.into(),
                effects: vec![],
                completes: true,
                experience: None,
                world_change: Some("server_save".into()),
            }],
        )
        .map_err(f::debug)?,
    );
    let missing_profile = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(1)?,
            request(bare_key, 11)?,
            bare,
        )
        .await?;
    assert!(
        matches!(missing_profile, O::Refused(_)),
        "private scope requires an explicit reset profile"
    );
    assert_eq!(h.root_revision().await?, "1");
    for (key, ordinal, revision) in [(reset.as_str(), 1, 1), (COMPLETE, 2, 2)] {
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
            matches!(result, O::Committed(_)),
            "personal episode cannot depend on an unrelated global event roll: {result:?}"
        );
    }
    let duplicate = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(3)?,
            request(COMPLETE, 3)?,
            catalogue.clone(),
        )
        .await?;
    assert!(matches!(duplicate, O::Refused(_)));
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let stale = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(3)?,
            request(COMPLETE, 4)?,
            catalogue.clone(),
        )
        .await?;
    assert!(
        matches!(stale, O::Refused(_)),
        "fresh restart required in the new save"
    );
    for (key, ordinal, revision) in [(reset.as_str(), 5, 3), (COMPLETE, 6, 4)] {
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
        assert!(matches!(result, O::Committed(_)), "{result:?}");
    }
    let copy = h
        .root
        .read_character_quest_state(&authority, f::fence(5)?.character_id)
        .await?;
    assert!(
        copy.save_event_quest_progress(QUEST)
            .ok_or("current personal episode")?
            .completed
    );
    assert_eq!(
        copy.save_event_quest_progress(QUEST)
            .ok_or("current personal episode")?
            .reset_epoch,
        2
    );
    let decisions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_change_days")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        decisions, 0,
        "personal daily admission must not activate physical World changes"
    );
    Ok(())
}
