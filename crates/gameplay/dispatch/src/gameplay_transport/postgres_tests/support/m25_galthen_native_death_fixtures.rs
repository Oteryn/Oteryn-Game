use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use super::fixture::{Harness, debug};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};

use crate::foundation::{CommandId, CommandRef};

use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};

use std::sync::Arc;

pub(super) const TRACK: &str = "oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/";

const QUEST: &str = "oteryn:quest.adventures_of_galthen_quest";

async fn value(h: &Harness, stage: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    )
    .bind(format!("{TRACK}{stage}"))
    .fetch_one(&h.pool)
    .await?)
}

async fn death(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    command: u64,
) -> TestResult {
    let target =
        super::qengine_kill_stories_fixtures::death_actor(owner, admitted, creature, command).await?;
    retire_death_actor(owner, target).await
}

async fn retire_death_actor(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    target: crate::foundation::ExactActorRef,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .companion_snapshot_including_dead(target)
            .map_err(debug)?
            .health,
        0
    );
    // The fixture drives lethal settlement without the AI lifecycle loop.
    // Retire its exact dead generation after both durable drains; corpse and
    // reward occurrences intentionally outlive the actor slot.
    runtime.remove_arena_opponent(target).map_err(debug)?;
    Ok(())
}

async fn predecessor(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    stage: &str,
    ordinal: u64,
) -> TestResult {
    let canonical = owner
        .quest_catalogue
        .as_ref()
        .ok_or("canonical Galthen catalogue")?;
    let key = format!("oteryn:quest-transition/m25-fixture/galthen-{stage}");
    let mut transitions = ["s5", "s8"]
        .into_iter()
        .map(|stage| {
            canonical
                .transition(&format!(
                    "oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/{stage}"
                ))
                .cloned()
                .ok_or("canonical M25 kill transition")
        })
        .collect::<Result<Vec<_>, _>>()?;
    for gate in ["s4", "s7"] {
        transitions.push(QuestTransition {
            world_change: None,
            key: format!("oteryn:quest-transition/m25-fixture/galthen-{gate}"),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: format!("{TRACK}{gate}"),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
        });
    }
    let tracks = ["s4", "s5", "s7", "s8"]
        .into_iter()
        .map(|stage| {
            canonical
                .track(&format!("{TRACK}{stage}"))
                .cloned()
                .ok_or("selected authored M25 track")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(canonical.content_revision(), tracks, transitions)
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "actual current Quest fence")?
        .ok_or("actual Quest fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key,
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(ordinal).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    // The shared quest scenario replaces the admitted runtime. Rebind only
    // its current durable Character lease before exercising native combat.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "current M25 combat fence")?
        .ok_or("current M25 combat fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("M25 combat actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    for (index, (creature, gate, stage)) in [
        ("oteryn:creature.ahau", "s4", "s5"),
        ("oteryn:creature.mitmah_vanguard", "s7", "s8"),
    ]
    .into_iter()
    .enumerate()
    {
        let command = 60000 + u64::try_from(index)? * 10;
        assert_eq!(value(h, gate).await?, 0);
        if stage == "s8" {
            assert_mitmah_reward_grant_boundary(owner, admitted, h, creature, command).await?;
        } else {
            death(owner, admitted, creature, command).await?;
        }
        super::m25_corpse_chain::assert_native_corpse_chain(&h.pool, creature).await?;
        assert_eq!(
            value(h, stage).await?,
            0,
            "actual death without the exact authored predecessor never counts"
        );
        predecessor(owner, admitted, gate, command + 1).await?;
        assert_eq!(value(h, gate).await?, 1);
        eprintln!(
            "M25_REAL_NATIVE_DEATH_REACHED creature={creature} predecessor={gate} target={stage}"
        );
        death(owner, admitted, creature, command + 2).await?;
        assert_eq!(
            value(h, stage).await?,
            1,
            "real native death progresses its selected authored stage once"
        );
        if stage == "s8" {
            // Emulate a pre-achievement character with the completed, retained s8 receipt.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
            for delete in [
                "DELETE FROM game_account_achievements WHERE achievement_key='oteryn:achievement/museum_goer'",
                "DELETE FROM game_account_achievement_grant_requests WHERE achievement_key='oteryn:achievement/museum_goer'",
            ] {
                let removed = sqlx::query(delete)
                    .execute(&mut *tx).await?;
                assert_eq!(removed.rows_affected(), 1);
            }
            tx.commit().await?;
        }
        death(owner, admitted, creature, command + 3).await?;
        assert_eq!(
            value(h, stage).await?,
            1,
            "duplicate projection/replay and another death cannot exceed the stage bound"
        );
        let completed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL)").bind(QUEST).fetch_one(&h.pool).await?;
        assert!(
            !completed,
            "native boss death never completes the quest or performs the final report"
        );
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1 AND NOT completes").bind(format!("oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/{stage}")).fetch_one(&h.pool).await?;
        assert_eq!(
            receipts, 1,
            "one actual death owns one non-completing stage receipt"
        );
        let museum_goer: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_achievement_grant_requests
             WHERE achievement_key='oteryn:achievement/museum_goer'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(museum_goer, i64::from(stage == "s8"));
    }
    Ok(())
}

async fn assert_mitmah_reward_grant_boundary(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    creature: &str,
    command: u64,
) -> TestResult {
    use crate::gameplay_transport::kill_reward::DrainOutcome;

    let rewards = async || -> TestResult<(i64, i64)> {
        Ok(sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_party_kill_xp_receipts WHERE creature_key=$1),
                    (SELECT count(*) FROM game_item_mint_receipts
                     WHERE loot_purpose_key='CORPSE_MATERIALIZATION' AND materialized_at IS NOT NULL)",
        )
        .bind(creature)
        .fetch_one(&h.pool)
        .await?)
    };
    let before = rewards().await?;
    sqlx::raw_sql(
        "CREATE FUNCTION reject_mitmah_award() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             IF NEW.achievement_key = 'oteryn:achievement/museum_goer' THEN
                 RAISE EXCEPTION 'injected Mitmah achievement failure';
             END IF;
             RETURN NEW;
         END $$;
         CREATE TRIGGER reject_mitmah_award BEFORE INSERT ON game_account_achievement_grant_requests
             FOR EACH ROW EXECUTE FUNCTION reject_mitmah_award()",
    )
    .execute(&h.pool)
    .await?;
    let target =
        super::qengine_kill_stories_fixtures::queue_death(owner, admitted, creature, command).await?;
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Deferred
    );
    assert_eq!(
        rewards().await?,
        before,
        "XP and corpse receipts must not survive a failed achievement grant"
    );
    sqlx::query("DROP TRIGGER reject_mitmah_award ON game_account_achievement_grant_requests")
        .execute(&h.pool)
        .await?;
    super::qengine_kill_stories_fixtures::drain_deaths(owner, admitted, &[]).await?;
    retire_death_actor(owner, target).await
}
