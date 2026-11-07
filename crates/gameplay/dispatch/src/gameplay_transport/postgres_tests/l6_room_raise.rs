//! Monotone room progress through the real PostgreSQL quest transaction owner.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition,
    world_callbacks::{Callback, Position},
};
use std::sync::Arc;
const TRACK: &str = "oteryn:quest-track/fixture-tomb-room/raise";
const QUEST: &str = "oteryn:quest.fixture-tomb-room-raise";
const SEED: &str = "oteryn:quest-transition/fixture-tomb-room-seed";
const APPLY: &str = "oteryn:quest-callback/fixture-tomb-room-raise";
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(
                name,
                Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "room raise scenario panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
#[test]
fn l6_room_raise_lower() -> TestResult {
    run("l6-room-raise-lower")
}
#[test]
fn l6_room_raise_equal() -> TestResult {
    run("l6-room-raise-equal")
}
#[test]
fn l6_room_raise_higher() -> TestResult {
    run("l6-room-raise-higher")
}
#[test]
fn l6_room_raise_refusal() -> TestResult {
    run("l6-room-raise-refusal")
}
#[test]
fn l6_room_raise_extreme() -> TestResult {
    run("l6-room-raise-extreme")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_room_raise version={version}");
    verify_lane_migrations(h).await?;
    let before = if name.ends_with("-lower") {
        -1
    } else if name.ends_with("-equal") {
        2
    } else if name.ends_with("-extreme") {
        i64::MAX
    } else {
        3
    };
    let refuses = name.ends_with("-refusal");
    let mut value = serde_json::to_value(
        oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .find(|callback| {
                callback
                    .key
                    .ends_with("/ancient-tombs-morguthis-seven-flame-gate")
            })
            .ok_or("source room callback")?,
    )?;
    value["key"] = APPLY.into();
    value["quest"] = QUEST.into();
    value["storage"] = serde_json::json!([{"track":TRACK,"minimum":if refuses {4} else {-1},"maximum":i64::MAX,"set":2,"at_least":true}]);
    let callback: Callback = serde_json::from_value(value)?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: TRACK.into(),
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max: i64::MAX,
            }],
            vec![
                QuestTransition {
world_change: None,
                    key: SEED.into(),
                    quest: QUEST.into(),
                    effects: vec![QuestEffect {
                        track: TRACK.into(),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::Set(before),
                    }],
                    completes: false,
                    experience: None,
                },
                oteryn_gameplay_quests::quest::world_callbacks::primary_transition(&callback)?,
            ],
        )
        .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    let seeded = commit(owner, admitted, SEED, 9000, Arc::clone(&catalogue)).await?;
    assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
    let initial = snapshot(h).await?;
    let result = commit(owner, admitted, APPLY, 9001, Arc::clone(&catalogue)).await?;
    if refuses {
        assert!(
            matches!(result, QuestTransitionOutcome::Refused(_)),
            "{result:?}"
        );
        assert_eq!(
            snapshot(h).await?,
            initial,
            "guard refusal cannot write any partial state or receipt"
        );
    } else {
        let QuestTransitionOutcome::Committed(receipt) = result else {
            return Err(format!("raise did not commit: {result:?}").into());
        };
        assert_eq!(receipt.changes.len(), 1);
        assert_eq!(receipt.changes[0].before, before);
        assert_eq!(receipt.changes[0].after, before.max(2));
        let stored: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(TRACK)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(stored, before.max(2));
        let committed = snapshot(h).await?;
        let replay = commit(owner, admitted, APPLY, 9001, catalogue).await?;
        assert_eq!(replay, QuestTransitionOutcome::AlreadyCommitted(receipt));
        assert_eq!(
            snapshot(h).await?,
            committed,
            "replay preserves original receipt and character revision"
        );
    }
    Ok(())
}
async fn commit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
    ordinal: u64,
    catalogue: Arc<QuestStateCatalogue>,
) -> TestResult<QuestTransitionOutcome> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "quest fence")?
        .ok_or("quest fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    slot.commit_quest_transition(
        owner.root,
        owner.character,
        owner.holder,
        fence,
        QuestTransitionRequest {
            transition_key: key.into(),
            cause: QuestCause::Command(CommandRef::new(
                admitted.game_session_id,
                CommandId::new(ordinal).map_err(debug)?,
            )),
        },
        catalogue,
    )
    .await
    .map_err(|e| format!("room raise write: {e:?}").into())
}
async fn snapshot(h: &Harness) -> TestResult<Vec<serde_json::Value>> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('tracks',(SELECT jsonb_agg(to_jsonb(t) ORDER BY track_key) FROM game_character_quest_tracks t),'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY committed_character_revision) FROM game_character_quest_receipts r),'revision',(SELECT character_revision::text FROM game_character_roots LIMIT 1))").fetch_all(&h.pool).await?)
}

async fn verify_lane_migrations(h: &Harness) -> TestResult {
    let expected: Vec<(i64, Vec<u8>)> = oteryn_gameplay_durability::GAME_MIGRATOR
        .iter()
        .filter(|migration| (1250..=1258).contains(&migration.version))
        .map(|migration| (migration.version, migration.checksum.to_vec()))
        .collect();
    assert_eq!(
        expected.len(),
        9,
        "all lane migrations use the assigned range"
    );
    let actual: Vec<(i64, Vec<u8>)> = sqlx::query_as(
        "SELECT version, checksum FROM _sqlx_migrations WHERE version BETWEEN 1250 AND 1258 AND success ORDER BY version",
    ).fetch_all(&h.pool).await?;
    assert_eq!(
        actual, expected,
        "actual PostgreSQL versions and SQLx checksums match the owning embedded migrator"
    );
    let obsolete: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM _sqlx_migrations WHERE version IN (1060,1061,1062,1063,1064,1065,1070,1071,1072)",
    ).fetch_one(&h.pool).await?;
    assert_eq!(
        obsolete, 0,
        "fresh lane qualification cannot install obsolete migration numbers"
    );
    eprintln!("POSTGRES_MIGRATION_REGISTRY_VERIFIED versions=1250-1258 checksums=9 obsolete=0");
    Ok(())
}
