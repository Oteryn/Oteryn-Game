//! Current native binding decisions precede the real Grimvale STEP and Hive USE.
use super::TestResult;
use super::fixture::{Harness, id};
use sha2::Digest;

#[test]
fn engworldchg_grimvale_binding_restart_and_native_step_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("engworldchg-grimvale", || {
        super::worldchg_grimvale::scenario_with_bindings(true)
    })
}

#[test]
fn engworldchg_hive_binding_reset_and_native_use_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("engworldchg-hive", || {
        super::worldchg_hive_egress::scenario_with_bindings(true)
    })
}

pub(super) async fn assert_binding_lifecycle(
    h: &Harness,
    world: crate::foundation::WorldId,
    epoch: u64,
) -> TestResult {
    let read = || {
        sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT bindings FROM game_world_change_native_bindings WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric",
    ).bind(world.as_bytes().as_slice()).bind(epoch.to_string())
    };
    let before = read().fetch_one(&h.pool).await?;
    // A fresh read reconstructs from PostgreSQL, without any process cache or RNG.
    let day = h
        .root
        .world_changes(world, epoch)
        .await?
        .ok_or("current bindings")?;
    for key in ["grimvale", "hive_born", "fire_from_the_earth"] {
        assert_eq!(
            h.root.world_change_active(world, key).await?,
            epoch != 0 && oteryn_gameplay_world_change::owner::is_active(&day, key),
            "the generic quest transition gate must observe the native activation for {key}"
        );
    }
    assert_eq!(read().fetch_one(&h.pool).await?, before);
    let other = crate::foundation::WorldId::decode(&id(44)).map_err(super::fixture::debug)?;
    assert!(h.root.world_changes(other, epoch).await?.is_none());
    assert!(h.root.world_changes(world, epoch + 1).await?.is_none());
    // Immutability applies to the runtime role as well as ordinary SQL writers.
    assert!(sqlx::query("UPDATE game_world_change_native_bindings SET bindings=$3 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(b"{}".as_slice())
        .execute(&h.pool).await.is_err());
    assert_eq!(h.root.world_changes(world, epoch).await?, Some(day));
    // Simulate storage loss with the admin fixture; ordinary runtime SQL cannot delete it.
    sqlx::query("ALTER TABLE game_world_change_native_bindings DISABLE TRIGGER game_world_change_native_bindings_immutable").execute(&h.pool).await?;
    sqlx::query("DELETE FROM game_world_change_native_bindings WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
    assert!(
        h.root.world_changes(world, epoch).await.is_err(),
        "a missing required binding cannot be redrawn or repaired by a gameplay read"
    );
    sqlx::query("INSERT INTO game_world_change_native_bindings(world_id,reset_epoch,bindings,bindings_digest) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4)")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&before)
        .bind(sha2::Sha256::digest(&before).as_slice()).execute(&h.pool).await?;
    sqlx::query("ALTER TABLE game_world_change_native_bindings ENABLE TRIGGER game_world_change_native_bindings_immutable").execute(&h.pool).await?;
    Ok(())
}

pub(super) async fn assert_native_quest_transition(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
    key: &str,
    quest: &str,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition,
    };
    let track = format!("oteryn:quest-progress/engworldchg/{key}");
    let transition = format!("oteryn:quest-transition/engworldchg/{key}");
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: track.clone(),
                quest: quest.into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![QuestTransition {
                key: transition.clone(),
                quest: quest.into(),
                effects: vec![QuestEffect {
                    track,
                    from: QuestComparison::Eq(0),
                    effect: QuestEffectKind::Set(1),
                }],
                completes: true,
                experience: None,
                world_change: Some(key.into()),
            }],
        )
        .map_err(super::fixture::debug)?,
    );
    let request = QuestTransitionRequest {
        transition_key: transition,
        cause: QuestCause::Command(crate::foundation::CommandRef::new(
            session,
            crate::foundation::CommandId::new(905).map_err(super::fixture::debug)?,
        )),
    };
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request.clone(),
            catalogue.clone(),
        )
        .await
        .map_err(super::fixture::debug)?;
    assert!(
        matches!(result, QuestTransitionOutcome::Committed(_)),
        "native activation must unlock the quest writer: {result:?}"
    );
    assert!(matches!(
        slot.commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request,
            catalogue
        )
        .await
        .map_err(super::fixture::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(format!("oteryn:quest-transition/engworldchg/{key}"))
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 1);
    Ok(())
}
