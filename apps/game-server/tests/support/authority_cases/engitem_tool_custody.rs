//! M13 physical tool custody uses the production MINT owner, with exact replay.
use super::engitem_reward_text::{Weights, command, equip_backpack};
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use oteryn_gameplay_quests::quest::reward_callbacks::{self, RewardCallback};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use std::sync::Arc;
#[path = "engitem_tools/capture.rs"]
mod capture;
#[path = "engitem_tools/corpse.rs"]
mod corpse;
#[path = "engitem_tools/live.rs"]
mod live;

#[test]
fn engitem_goblin_tools_mint_atomically_with_exact_replay() -> f::TestResult {
    run(0)
}

#[test]
fn engitem_goblin_capture_is_atomic_single_target_and_replayed() -> f::TestResult {
    run(1)
}

#[test]
fn engitem_goblin_corpse_sampling_is_source_bound_fresh_and_replayed() -> f::TestResult {
    run(2)
}

fn run(mode: u8) -> f::TestResult {
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
                    let h = f::Harness::create(admin, "engitem_tool_custody", true).await?;
                    let result = exercise(&h, mode).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGITEM_PG_EXECUTED goblin_tool_custody mode={} duration={:?}",
                        mode,
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "tool custody PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness, mode: u8) -> f::TestResult {
    let callback: RewardCallback = serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-reward/engitem-tool-custody", "quest":"oteryn:quest.engitem_tool_custody",
        "npc":"oteryn:npc.fixture", "keywords":["yes"], "topic":0,"next_topic":0,
        "storage":[{"track":"oteryn:quest-progress/engitem-tool-custody","minimum":0,"maximum":0,"set":1}],
        "take":[],"give":{"item":"oteryn:item.tibia.i17673","count":1,"contents":[],"additional":[{"item":"oteryn:item.tibia.i17670","count":1},{"item":"oteryn:item.tibia.i17674","count":1},{"item":"oteryn:item.tibia.i17669","count":1}]},
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
    let (controller, pin) =
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
    let absent:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(
        absent,
        (0, 0),
        "no backpack: no partial tool or quest progress"
    );
    equip_backpack(h, &authority, native, &weights).await?;
    assert!(matches!(
        apply(callback.clone(), catalogue.clone())
            .await
            .map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    let read = || {
        sqlx::query_as::<_,(String,i64)>("SELECT definition_production_key,quantity FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i17673','oteryn:item.tibia.i17674') ORDER BY definition_production_key").fetch_all(&h.pool)
    };
    let expected = vec![
        ("oteryn:item.tibia.i17673".into(), 1),
        ("oteryn:item.tibia.i17674".into(), 1),
    ];
    assert_eq!(read().await?, expected);
    let atomic: bool = sqlx::query_scalar("SELECT s.xmin=q.xmin AND s.xmin=m.xmin FROM game_quest_reward_sources s JOIN game_reward_claim_mint_receipts m ON m.transaction_id=s.mint_transaction_id JOIN game_character_quest_receipts q ON (q.character_id,q.cause_id,q.cause_ordinal,q.transition_key)=(s.character_id,s.cause_id,s.cause_ordinal,s.transition_key)").fetch_one(&h.pool).await?;
    assert!(
        atomic,
        "source, primary tool MINT and quest progress share a transaction"
    );
    assert!(matches!(
        apply(callback, catalogue).await.map_err(f::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(
        read().await?,
        expected,
        "exact replay never remints either tool"
    );
    if mode == 1 {
        capture::capture(h, &authority, &pin, native).await?;
    } else if mode == 2 {
        corpse::sample(h, &authority, &pin, native).await?;
    } else {
        live::sample(h, &authority, &pin, native).await?;
    }
    Ok(())
}
