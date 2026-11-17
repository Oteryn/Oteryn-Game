//! Real lever -> selection -> Start -> native lethal -> individual record -> highscore dispatch.
use super::*;
use oteryn_protocol_oteryn::boss_difficulty::{self as difficulty, Reply, Request};

async fn action(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    request: Request,
) -> TestResult<Reply> {
    let id = admitted.continuity.next_command_id;
    let payload = difficulty::encode_request(&request).map_err(debug)?;
    let (status, bytes) =
        super::super::boss_difficulty::exchange(owner, admitted, id, &payload).await?;
    assert_eq!(status, CommandStatus::Accepted);
    difficulty::decode_reply(&bytes).map_err(debug)
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    boss: &str,
    policy: &crate::foundation::CompiledCreaturePolicy,
) -> TestResult {
    let id = admitted.continuity.next_command_id;
    let body = wire::encode_boss_key(boss).map_err(debug)?;
    let (status, bytes) = super::super::boss_difficulty::exchange_kind(
        owner,
        admitted,
        id,
        &body,
        wire::COMMAND_TYPE_BOSS_LEVER,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let Reply::Window {
        context,
        personal_highest,
        unlocked_max,
        maximum_difficulty,
        selected_difficulty,
        ..
    } = difficulty::decode_reply(&bytes).map_err(debug)?
    else {
        return Err("lever must open difficulty window".into());
    };
    assert_eq!(
        (
            personal_highest,
            unlocked_max,
            maximum_difficulty,
            selected_difficulty
        ),
        (0, 1, 28, 1)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_room_activities")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    assert_eq!(
        action(
            owner,
            admitted,
            Request::Start {
                context,
                difficulty: 2
            }
        )
        .await?,
        Reply::Refused {
            context: Some(context),
            reason: difficulty::Refusal::DifficultyLocked
        }
    );
    assert_eq!(
        action(
            owner,
            admitted,
            Request::Start {
                context,
                difficulty: 0
            }
        )
        .await?,
        Reply::Refused {
            context: Some(context),
            reason: difficulty::Refusal::DifficultyLocked
        }
    );
    let before = *admitted;
    assert_eq!(
        action(
            owner,
            admitted,
            Request::Start {
                context,
                difficulty: 1
            }
        )
        .await?,
        Reply::Closed { context }
    );
    assert_ne!(before.game_session_id, admitted.game_session_id);
    assert!(matches!(
        admitted
            .item_fence
            .ok_or("destination fence")?
            .runtime_scope,
        RuntimeScopeRefV1::Instance { .. }
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i16>("SELECT difficulty FROM game_boss_room_activities")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    let destination = owner
        .resolve_runtime_view(*admitted)
        .await
        .map_err(|_| "difficulty destination")?
        .ok_or("new owner")?;
    nonreward::assert_defeat_and_return(h, owner, &destination, *admitted, policy).await?;
    let character = before.item_fence.ok_or("origin fence")?.character_id;
    let highest: i16 = sqlx::query_scalar("SELECT highest_defeated FROM game_character_boss_difficulty_records WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2")
        .bind(character.as_bytes().as_slice()).bind(boss).fetch_one(&h.pool).await?;
    assert_eq!(highest, 1);
    *admitted = owner.activity_runtimes.current_connection(*admitted);
    let Reply::Window {
        personal_highest,
        unlocked_max,
        ..
    } = action(
        owner,
        admitted,
        Request::OpenWithProgress {
            boss_key: boss.into(),
        },
    )
    .await?
    else {
        return Err("reopened victory window".into());
    };
    assert_eq!((personal_highest, unlocked_max), (1, 2));
    use crate::durability::highscore_snapshot::{SnapshotCatalogues, publish_in_transaction};
    let catalogues = SnapshotCatalogues::activated(std::sync::Arc::new(
        crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?,
    ))
    .map_err(debug)?;
    let mut tx = h.pool.begin().await?;
    publish_in_transaction(&mut tx, admitted.world_id, &catalogues)
        .await
        .map_err(debug)?;
    tx.commit().await?;
    let query = oteryn_protocol_oteryn::highscores::HighscoresQuery {
        world: None,
        details: false,
        category: 16,
        vocation: 0,
        page: 1,
        snapshot_id: 0,
    };
    let id = admitted.continuity.next_command_id;
    let payload =
        oteryn_protocol_oteryn::highscores::encode_highscores_query(query).map_err(debug)?;
    let (status, bytes) =
        super::super::boss_difficulty::exchange_kind(owner, admitted, id, &payload, 14).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let result =
        oteryn_protocol_oteryn::highscores::decode_highscores_result(&bytes).map_err(debug)?;
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].value, 1);
    eprintln!("BOSS_DIFFICULTY_VICTORY_PHOSPHORUS POSTGRES_CASE_COMPLETED");
    Ok(())
}
