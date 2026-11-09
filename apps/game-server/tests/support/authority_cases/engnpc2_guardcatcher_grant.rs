//! Native Guardcatcher custody through the existing fenced quest MINT caller.
//! This qualifies the tool, without asserting capture probability or NPC talk admission.
use crate::bestiary_postgres_harness as f;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use oteryn_gameplay_quests::quest::{
    QuestEffectKind, QuestStateCatalogue, QuestTrack, reward_callbacks,
};
use std::sync::Arc;

#[test]
fn engnpc2_guardcatcher_postgres_native_grant_and_replay() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        f::runtime().map_err(f::debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = f::Harness::create(admin, "engnpc2_guardcatcher", true).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("PG_EXECUTED engnpc2_guardcatcher_postgres_native_grant_and_replay duration={:?}", started.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(f::debug)
    })?.join().map_err(|_| "Guardcatcher PG thread failed")?.map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    const TOOL: &str = "oteryn:item.tibia.i17669";
    const TRACK: &str = "oteryn:quest-progress/engnpc2_guard_custody_fixture";
    let callback: reward_callbacks::RewardCallback = serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-reward/engnpc2-guard-custody-fixture",
        "quest":"oteryn:quest.engnpc2_guard_custody_fixture", "npc":"oteryn:npc.rafzan",
        "keywords":["guards"], "topic":0, "next_topic":0,
        "storage":[{"track":TRACK,"minimum":0,"maximum":0,"set":1}],
        "take":[], "give":{"item":TOOL,"count":1,"contents":[]},
        "outfit":null,"achievement":null,"reply":["Take this bow."],
        "sources":["https://www.tibiawiki.com.br/wiki/Task:Rafzan#Guardian_Angel_-_Kind_Of"]
    }))?;
    let mut transition = reward_callbacks::primary_transition(&callback).map_err(f::debug)?;
    // Only this isolated fixture owns the track; the global bound-operation registry stays closed.
    for effect in &mut transition.effects {
        if let QuestEffectKind::SetWithBinding { value, .. } = effect.effect {
            effect.effect = QuestEffectKind::Set(value);
        }
    }
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: TRACK.into(),
                quest: callback.quest.clone(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![transition],
        )
        .map_err(f::debug)?,
    );
    let (controller, _pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native content")?;
    let weights = super::engitem_reward_text::Weights(native);
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let request = QuestTransitionRequest {
        transition_key: callback.key.clone(),
        cause: QuestCause::Command(super::engitem_reward_text::command(13200)?),
    };
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
        creature_use: None,
        peer_use: None,
    };
    let fence = f::fence(1)?;
    let apply = || {
        h.root.commit_fixture_quest_reward(
            &authority,
            &h.node,
            fence,
            request.clone(),
            catalogue.clone(),
            admission,
            callback.clone(),
        )
    };
    assert!(matches!(
        apply().await.map_err(f::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    assert_eq!(h.count("game_character_quest_receipts").await?, 0);
    super::engitem_reward_text::equip_backpack(h, &authority, native, &weights).await?;
    let result = apply().await.map_err(f::debug)?;
    assert!(
        matches!(result, QuestTransitionOutcome::Committed(_)),
        "native Guardcatcher grant: {result:?}"
    );
    let minted:(i64, i64) = sqlx::query_as("SELECT count(*),coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(TOOL).fetch_one(&h.pool).await?;
    assert_eq!(minted, (1, 1));
    let atomic:i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_character_quest_receipts q ON m.xmin=q.xmin WHERE q.transition_key=$1").bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(
        atomic, 1,
        "tool MINT and grant progress share one transaction"
    );
    assert!(matches!(
        apply().await.map_err(f::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(h.count("game_reward_claim_mint_receipts").await?, 1);
    assert_eq!(h.root_revision().await?, "2");
    Ok(())
}
