//! QSTORE reuses Quest tracks, callback lowering and the session-fenced writer.
use crate::bestiary_postgres_harness as f;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffectKind, QuestStateCatalogue, QuestTrack, world_callbacks,
};
use std::sync::Arc;

#[test]
fn qstore_storage_conditions_are_durable_and_session_generation_fenced() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "qstore_storage_conditions", true).await?;
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!("QSTORE_POSTGRES_EXECUTED duration={:?}", started.elapsed());
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "QSTORE PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let track = "oteryn:quest-progress/qstore/flags";
    let mut callback = world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == oteryn_gameplay_quests::quest::rotten_blood::SACRIFICE)
        .ok_or("Rotten Blood callback")?;
    callback.key = "oteryn:quest-transition/qstore/flags".into();
    callback.native_rule = None;
    callback.storage.clear();
    callback.state_operations = serde_json::from_value(serde_json::json!([{
        "track":track,"from":{"kind":"bits_set","mask":5},"effect":{"kind":"add","value":0}
    }]))?;
    let gate = world_callbacks::primary_transition(&callback)?;
    let mut seed = gate.clone();
    seed.key = "oteryn:quest-transition/qstore/seed".into();
    seed.effects[0].from = QuestComparison::Any;
    seed.effects[0].effect = QuestEffectKind::Set(5);
    let mut clear = gate.clone();
    clear.key = "oteryn:quest-transition/qstore/clear".into();
    clear.effects[0].from = QuestComparison::BitsClear(2);
    let mut denied_clear = clear.clone();
    denied_clear.key = "oteryn:quest-transition/qstore/denied-clear".into();
    denied_clear.effects[0].from = QuestComparison::BitsClear(1);
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: track.into(),
                quest: callback.quest.clone(),
                initial: 0,
                min: 0,
                max: i64::MAX,
            }],
            vec![
                gate.clone(),
                seed.clone(),
                clear.clone(),
                denied_clear.clone(),
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
    for (number, key, revision, allowed) in [
        (1, &gate.key, 1, false),
        (2, &seed.key, 1, true),
        (3, &gate.key, 2, true),
        (4, &clear.key, 3, true),
        (5, &denied_clear.key, 4, false),
    ] {
        let outcome = h
            .root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(revision)?,
                request(key, number)?,
                catalogue.clone(),
            )
            .await
            .map_err(f::debug)?;
        assert_eq!(
            matches!(outcome, QuestTransitionOutcome::Committed(_)),
            allowed
        );
        if !allowed {
            assert!(matches!(outcome, QuestTransitionOutcome::Refused(_)));
        }
    }
    let mut stale = f::fence(4)?;
    stale.connection_generation = ConnectionGeneration::new(2).map_err(f::debug)?;
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                stale,
                request(&gate.key, 6)?,
                catalogue
            )
            .await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    let rows: (i64,i64) = sqlx::query_as("SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),(SELECT count(*) FROM game_character_quest_receipts)")
        .bind(track).fetch_one(&h.pool).await?;
    assert_eq!(rows, (5, 3));
    for (kind, mask, before, allowed) in [
        ("bits_set", 5, 1, false),
        ("bits_set", 5, 5, true),
        ("bits_clear", 2, 5, true),
        ("bits_clear", 1, 5, false),
        ("bits_set", 0, 5, false),
    ] {
        let plan = serde_json::json!({"storage":[],"state_ops":[{"track":track,"from":{"kind":kind,"mask":mask},"effect":{"kind":"add","value":0}}]});
        let valid: bool = sqlx::query_scalar("SELECT game_quest_reward_effects_valid($1,$2,$3,$4)")
            .bind(plan)
            .bind(vec![track])
            .bind(vec![i64::from(before)])
            .bind(vec![i64::from(before)])
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(valid, allowed);
    }
    // QSTORE must preserve migration 1600's capacity and 2061's checked floor arm.
    let keys: Vec<_> = (0..16)
        .map(|index| format!("oteryn:quest-progress/qstore/flag-{index}"))
        .collect();
    let operations: Vec<_> = keys
        .iter()
        .map(|key| {
            serde_json::json!({
                "track":key,"from":{"kind":"bits_set","mask":1},
                "effect":{"kind":"add_at_least","value":-1,"minimum":0}
            })
        })
        .collect();
    let valid: bool = sqlx::query_scalar("SELECT game_quest_reward_effects_valid($1,$2,$3,$4)")
        .bind(serde_json::json!({"storage":[],"state_ops":operations}))
        .bind(keys)
        .bind(vec![5_i64; 16])
        .bind(vec![4_i64; 16])
        .fetch_one(&h.pool)
        .await?;
    assert!(
        valid,
        "existing sixteen-effect checked-floor plans remain valid"
    );
    Ok(())
}
fn request(key: &str, number: u64) -> f::TestResult<QuestTransitionRequest> {
    Ok(QuestTransitionRequest {
        transition_key: key.into(),
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
            CommandId::new(number).map_err(f::debug)?,
        )),
    })
}
