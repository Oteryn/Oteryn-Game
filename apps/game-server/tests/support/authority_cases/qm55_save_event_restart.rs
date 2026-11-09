//! An explicit reset profile scopes mission state without resetting private balances.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison as C, QuestEffect, QuestEffectKind as E, QuestStateCatalogue, QuestTrack,
    QuestTransition,
};
const QUEST: &str = "oteryn:quest.qm55_restart_event";
const TRACK: &str = "oteryn:quest-progress/qm55/restart-event";
const BALANCE: &str = "oteryn:quest-progress/qm55/persistent-balance";
const RESET: &str = "oteryn:quest-transition/save-event-restart/qm55_restart_event";
const SEED: &str = "oteryn:quest-transition/qm55/balance-seed";
const BEGIN: &str = "oteryn:quest-transition/qm55/restart-begin";
const FINISH: &str = "oteryn:quest-transition/qm55/restart-finish";
#[test]
fn qm55_authored_save_restart_refuses_mid_occurrence_reset_and_stale_objectives() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "qm55_save_restart", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M55_PG_EXECUTED save_restart duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M55 save restart worker failed")?
        .map_err(Into::into)
}
async fn exercise(h: &f::Harness) -> f::TestResult {
    let world = f::scope()?.world_id();
    for epoch in 1..=3_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')").bind(world.as_bytes().as_slice()).bind(epoch).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
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
    let effect = |track: &str, from, value| QuestEffect {
        track: track.into(),
        from,
        effect: E::Set(value),
    };
    let transition = |key: &str, effects, completes, world_change| QuestTransition {
        key: key.into(),
        quest: QUEST.into(),
        effects,
        completes,
        experience: None,
        world_change,
    };
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![
                QuestTrack {
                    key: TRACK.into(),
                    quest: QUEST.into(),
                    initial: 0,
                    min: 0,
                    max: 2,
                },
                QuestTrack {
                    key: BALANCE.into(),
                    quest: QUEST.into(),
                    initial: 0,
                    min: 0,
                    max: 100,
                },
            ],
            vec![
                transition(
                    SEED,
                    vec![effect(BALANCE, C::Between(0, 0), 49)],
                    false,
                    None,
                ),
                transition(
                    RESET,
                    vec![effect(TRACK, C::Any, 0)],
                    false,
                    Some("grimvale".into()),
                ),
                transition(
                    BEGIN,
                    vec![effect(TRACK, C::Between(0, 0), 1)],
                    false,
                    Some("grimvale".into()),
                ),
                transition(
                    FINISH,
                    vec![effect(TRACK, C::Between(1, 1), 2)],
                    true,
                    Some("grimvale".into()),
                ),
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
    for (key, revision, ordinal) in [(SEED, 1, 1), (RESET, 2, 2), (BEGIN, 3, 3)] {
        assert!(matches!(
            h.root
                .commit_character_quest_transition(
                    &authority,
                    &h.node,
                    f::fence(revision)?,
                    request(key, ordinal)?,
                    catalogue.clone()
                )
                .await?,
            QuestTransitionOutcome::Committed(_)
        ));
    }
    let duplicate = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(4)?,
            request(RESET, 4)?,
            catalogue.clone(),
        )
        .await?;
    assert!(
        matches!(duplicate, QuestTransitionOutcome::Refused(_)),
        "reset cannot discard partial progress in the same save: {duplicate:?}"
    );
    assert_eq!(h.root_revision().await?, "4");
    for epoch in 2..=3_i64 {
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$2 WHERE world_id=encode($1,'hex')::uuid").bind(world.as_bytes().as_slice()).bind(epoch).execute(&h.pool).await?;
        h.root
            .world_changes(world, epoch as u64)
            .await?
            .ok_or("next day")?;
        let revision = if epoch == 2 { 4 } else { 7 };
        let stale = h
            .root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(revision)?,
                request(FINISH, 20 + epoch as u64)?,
                catalogue.clone(),
            )
            .await?;
        assert!(
            matches!(stale, QuestTransitionOutcome::Refused(_)),
            "new save requires fresh restart"
        );
        for (offset, key) in [RESET, BEGIN, FINISH].into_iter().enumerate() {
            let ordinal = 100 + epoch as u64 * 10 + offset as u64;
            assert!(
                matches!(
                    h.root
                        .commit_character_quest_transition(
                            &authority,
                            &h.node,
                            f::fence(revision + offset as u64)?,
                            request(key, ordinal)?,
                            catalogue.clone()
                        )
                        .await?,
                    QuestTransitionOutcome::Committed(_)
                ),
                "{key}"
            );
        }
        let copy = h
            .root
            .read_character_quest_state(&authority, f::fence(1)?.character_id)
            .await?;
        assert_eq!(copy.tracks().get(BALANCE), Some(&49));
        assert_eq!(copy.tracks().get(TRACK), Some(&2));
        let progress = copy
            .save_event_quest_progress(QUEST)
            .ok_or("current save missing")?;
        assert_eq!(progress.reset_epoch, epoch as u64);
        assert!(progress.completed);
    }
    Ok(())
}
