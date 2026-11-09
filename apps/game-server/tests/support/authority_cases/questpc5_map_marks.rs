//! M16: native map marker and dialogue progress share one fenced receipt.
use crate::bestiary_postgres_harness as f;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::reward_callbacks::{self, RewardCallback};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use std::sync::Arc;

const KEY: &str = "oteryn:quest-reward/targuna/sterling-mark-treasure";
const QUEST: &str = "oteryn:quest.targuna_quest";
const MARKED: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/long_lost_treasure/treasure_marked";
const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/long_lost_treasure/mission";
const MORLA: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/long_lost_treasure/morla_done";
const READY: &str = "oteryn:quest-transition/questpc5-marker-fixture/morla-ready";

#[test]
fn questpc5_personal_map_mark_is_fenced_atomic_and_replayed() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "questpc5_map_marks", true).await?;
                    let version: String = sqlx::query_scalar("SHOW server_version")
                        .fetch_one(&h.pool)
                        .await?;
                    eprintln!("M16_POSTGRES_EXECUTED server={version}");
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    eprintln!(
                        "M16_PG_COMPLETED duration={:?} success={}",
                        started.elapsed(),
                        result.is_ok() && cleanup.is_ok()
                    );
                    result?;
                    cleanup?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M16 PG thread failed")?
        .map_err(Into::into)
}

struct NoItems<'a>(&'a NativeGameplayState);
impl ItemWeightLookup for NoItems<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn weight_of(&self, _: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        Err(ItemWeightRefused::Unknown)
    }
}
fn command(n: u64) -> f::TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
        CommandId::new(n).map_err(f::debug)?,
    ))
}
async fn counts(h: &f::Harness) -> f::TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_map_marks),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?)
}
async fn exercise(h: &f::Harness) -> f::TestResult {
    let callback: RewardCallback = reward_callbacks::load()
        .map_err(f::debug)?
        .into_iter()
        .find(|row| row.key == KEY)
        .ok_or("production Sterling map callback")?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            [MISSION, MARKED, MORLA]
                .into_iter()
                .map(|key| QuestTrack {
                    key: key.into(),
                    quest: QUEST.into(),
                    initial: if key == MISSION { 3 } else { 0 },
                    min: 0,
                    max: if key == MISSION { 5 } else { 1 },
                })
                .collect(),
            vec![
                reward_callbacks::primary_transition(&callback).map_err(f::debug)?,
                QuestTransition {
                    key: READY.into(),
                    quest: QUEST.into(),
                    effects: vec![QuestEffect {
                        track: MORLA.into(),
                        from: QuestComparison::Eq(0),
                        effect: QuestEffectKind::Set(1),
                    }],
                    completes: false,
                    experience: None,
                    world_change: None,
                },
            ],
        )
        .map_err(f::debug)?,
    );
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = NoItems(native);
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
        creature_use: None,
        peer_use: None,
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let request = QuestTransitionRequest {
        transition_key: KEY.into(),
        cause: QuestCause::Command(command(9401)?),
    };
    assert!(matches!(
        h.root
            .commit_fixture_quest_reward(
                &authority,
                &h.node,
                f::fence(1)?,
                request.clone(),
                catalogue.clone(),
                admission,
                callback.clone()
            )
            .await
            .map_err(f::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(
        counts(h).await?,
        (0, 0),
        "Morla guard refuses without progress or marker"
    );
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                f::fence(1)?,
                QuestTransitionRequest {
                    transition_key: READY.into(),
                    cause: QuestCause::Command(command(9400)?)
                },
                catalogue.clone()
            )
            .await
            .map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    let mut invalid = callback.clone();
    invalid.map_mark.as_mut().ok_or("marker")?.text.clear();
    assert!(matches!(
        h.root
            .commit_fixture_quest_reward(
                &authority,
                &h.node,
                f::fence(2)?,
                request.clone(),
                catalogue.clone(),
                admission,
                invalid
            )
            .await,
        Err(CharacterProgressionError::InvalidInput)
    ));
    assert_eq!(counts(h).await?, (0, 1));
    let mut stale = f::fence(2)?;
    stale.character_lease_generation += 1;
    assert!(matches!(
        h.root
            .commit_fixture_quest_reward(
                &authority,
                &h.node,
                stale,
                request.clone(),
                catalogue.clone(),
                admission,
                callback.clone()
            )
            .await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    assert_eq!(counts(h).await?, (0, 1));
    assert!(matches!(
        h.root
            .commit_fixture_quest_reward(
                &authority,
                &h.node,
                f::fence(2)?,
                request.clone(),
                catalogue.clone(),
                admission,
                callback.clone()
            )
            .await
            .map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    let snapshot = h
        .root
        .read_character_quest_map_marks(&authority, &h.node, f::fence(3)?)
        .await
        .map_err(f::debug)?;
    assert_eq!(snapshot.character_revision, 3);
    assert_eq!(snapshot.marks.len(), 1);
    assert_eq!(snapshot.marks[0].marker.text, "Treasure");
    assert!(
        h.root
            .read_character_quest_map_marks(&authority, &h.node, f::fence(2)?)
            .await
            .is_err(),
        "stale read fence must not publish facts"
    );
    let row:(i32,i32,i16,String,String,bool,bool)=sqlx::query_as("SELECT m.x,m.y,m.floor,m.kind,m.label,m.created_xact_id=r.created_xact_id AND r.created_xact_id=s.created_xact_id,m.world_id=encode($1,'hex')::uuid FROM game_character_quest_map_marks m JOIN game_character_quest_receipts r ON (r.character_id,r.committed_character_revision)=(m.character_id,m.committed_character_revision) JOIN game_quest_reward_sources s ON (s.character_id,s.cause_id,s.cause_ordinal,s.transition_key)=(r.character_id,r.cause_id,r.cause_ordinal,r.transition_key)").bind(f::scope()?.world_id().as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        row,
        (
            31948,
            31924,
            -7,
            "cross".into(),
            "Treasure".into(),
            true,
            true
        )
    );
    assert_eq!(counts(h).await?, (1, 2));
    assert!(matches!(
        h.root
            .commit_fixture_quest_reward(
                &authority,
                &h.node,
                f::fence(2)?,
                request,
                catalogue.clone(),
                admission,
                callback.clone()
            )
            .await
            .map_err(f::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(counts(h).await?, (1, 2));
    let fresh = QuestTransitionRequest {
        transition_key: KEY.into(),
        cause: QuestCause::Command(command(9402)?),
    };
    assert!(matches!(
        h.root
            .commit_fixture_quest_reward(
                &authority,
                &h.node,
                f::fence(3)?,
                fresh,
                catalogue,
                admission,
                callback
            )
            .await
            .map_err(f::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(counts(h).await?, (1, 2));
    let error = sqlx::query("UPDATE game_character_quest_map_marks SET label='Other'")
        .execute(&h.pool)
        .await
        .err()
        .ok_or("immutable marker update unexpectedly allowed")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    let label: String = sqlx::query_scalar("SELECT label FROM game_character_quest_map_marks")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(label, "Treasure");
    Ok(())
}
