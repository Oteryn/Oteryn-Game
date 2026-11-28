//! Literal source text composes the existing Quest and backpack MINT transaction.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use oteryn_gameplay_quests::quest::reward_callbacks::{self, RewardCallback};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use std::sync::Arc;

#[path = "../engitem_reward_fixture.rs"]
mod reward_fixture;
pub(super) use reward_fixture::{Weights, command, definition, equip_backpack};

#[test]
fn engitem_literal_reward_text_is_atomic_refused_and_replayed() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "engitem_reward_text", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGITEM_PG_EXECUTED literal_reward_text duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "reward text PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let body = "A source-authored letter.\nKeep the literal café text.\n";
    let callback: RewardCallback = serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-reward/engitem-readable", "quest":"oteryn:quest.engitem_readable",
        "npc":"oteryn:npc.fixture", "keywords":["yes"], "topic":0,"next_topic":0,
        "storage":[{"track":"oteryn:quest-progress/engitem-readable","minimum":0,"maximum":0,"set":1}],
        "take":[],"give":{"item":"oteryn:item.tibia.i2814","count":1,"contents":[],"text":body},
        "outfit":null,"achievement":null,"reply":["Granted."],"sources":["engine fixture"]
    }))?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: callback.storage[0].track.clone(),
                quest: callback.quest.clone(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![reward_callbacks::primary_transition(&callback).map_err(f::debug)?],
        )
        .map_err(f::debug)?,
    );
    let request = QuestTransitionRequest {
        transition_key: callback.key.clone(),
        cause: QuestCause::Command(command(9400)?),
    };
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = Weights(native);
    let admission = QuestRewardAdmission {
        selected_fish: None,
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
    let quest_fence = f::fence(1)?;
    let apply = |callback: RewardCallback, catalogue: Arc<QuestStateCatalogue>| {
        h.root.commit_fixture_quest_reward(
            &authority,
            &h.node,
            quest_fence,
            request.clone(),
            catalogue,
            admission,
            callback,
        )
    };
    assert!(matches!(
        apply(callback.clone(), catalogue.clone())
            .await
            .map_err(f::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    let absent:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_item_texts),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(
        absent,
        (0, 0),
        "no backpack: no partial text or quest progress"
    );
    equip_backpack(h, &authority, native, &weights).await?;
    assert!(matches!(
        apply(callback.clone(), catalogue.clone())
            .await
            .map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    let read = || {
        sqlx::query_as::<_,(String,i64,i64)>("SELECT t.text,(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_character_quest_receipts) FROM game_item_texts t").fetch_one(&h.pool)
    };
    assert_eq!(read().await?, (body.into(), 1, 1));
    let atomic: bool = sqlx::query_scalar("SELECT m.root_text=t.text AND (s.plan #>> '{give,text}')=t.text AND t.writer_character_id IS NULL AND t.text_revision=1 AND t.xmin=m.xmin AND m.xmin=s.xmin AND s.xmin=q.xmin FROM game_item_texts t JOIN game_reward_claim_mint_receipts m USING(item_instance_id) JOIN game_quest_reward_sources s ON s.mint_transaction_id=m.transaction_id JOIN game_character_quest_receipts q ON (q.character_id,q.cause_id,q.cause_ordinal,q.transition_key)=(s.character_id,s.cause_id,s.cause_ordinal,s.transition_key)")
        .fetch_one(&h.pool).await?;
    assert!(
        atomic,
        "exact text, source, MINT and progress share one transaction"
    );
    assert!(matches!(
        apply(callback, catalogue).await.map_err(f::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(
        read().await?,
        (body.into(), 1, 1),
        "exact replay never remints or rewrites text"
    );
    Ok(())
}
