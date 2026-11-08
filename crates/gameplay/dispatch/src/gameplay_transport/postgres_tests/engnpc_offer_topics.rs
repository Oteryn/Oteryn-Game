//! Read-only source offers share the real NPC conversation and retain quest guards.
use super::fixture::Harness;
use super::{ComposedFreshAdmission, Dependencies, TestResult};
use crate::gameplay_transport::connection::UseCommand;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

#[test]
fn engnpc_offers_respect_guards_across_dialogue_topics() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .name("engnpc-offer-topics".into())
        .stack_size(256 << 20)
        .spawn(|| super::npc_paid_travel::offer_scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "offer topic thread failed")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED engnpc_offers_respect_guards_across_dialogue_topics duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
    npc: EntityRef,
    h: &Harness,
) -> TestResult {
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    let offer = |topic, next_topic| {
        serde_json::json!({
            "npc":"oteryn:npc.captain_bluebear", "keywords":["fixture offer"],
            "topic":topic, "next_topic":next_topic, "storage":[], "reply":["fixture reply"],
        })
    };
    assert!(apply(owner, dependencies, npc, 9901, offer(0, 7)).await?);
    assert!(
        !apply(owner, dependencies, npc, 9902, offer(0, 0)).await?,
        "ordinary offers must retain topic guards"
    );
    let mut guarded = offer(0, 0);
    guarded["any_topic"] = true.into();
    guarded["requires"] = serde_json::json!([{"track":"oteryn:quest-progress/fixture/unavailable", "minimum":1, "maximum":1}]);
    assert!(
        !apply(owner, dependencies, npc, 9903, guarded).await?,
        "any-topic offers must retain quest eligibility"
    );
    let mut storage_guarded = offer(0, 0);
    storage_guarded["any_topic"] = true.into();
    storage_guarded["storage"] = serde_json::json!([{
        "track":"oteryn:quest-progress/fixture/unavailable",
        "minimum":1,"maximum":1,"set":1,
    }]);
    assert!(
        !apply(owner, dependencies, npc, 9906, storage_guarded).await?,
        "any-topic offers must retain storage guards"
    );
    let mut any_topic = offer(0, 0);
    any_topic["any_topic"] = true.into();
    assert!(
        apply(owner, dependencies, npc, 9904, any_topic).await?,
        "any-topic offer refused at dialogue topic seven"
    );
    assert!(
        apply(owner, dependencies, npc, 9905, offer(0, 0)).await?,
        "the selected next topic must be committed"
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        before, after,
        "read-only offers must not invent quest receipts"
    );
    Ok(())
}

async fn apply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
    npc: EntityRef,
    command_id: u64,
    offer: serde_json::Value,
) -> TestResult<bool> {
    let fence = owner
        .current_quest_fence(dependencies.admitted.game_session_id)
        .await
        .map_err(|_| "offer fence")?
        .ok_or("offer session")?
        .item_fence();
    let source = serde_json::json!({"schema":1,"offers":[offer]}).to_string();
    let outcome = crate::gameplay_transport::npc_command::fixture_apply_reward_offer(
        owner,
        dependencies.actor,
        UseCommand {
            game_session_id: dependencies.admitted.game_session_id,
            command_id,
            item_fence: Some(fence),
        },
        NpcTalkIntent {
            npc_actor: npc,
            text: "fixture offer".into(),
        },
        &source,
    )
    .await;
    let Some(outcome) = outcome else {
        return Ok(false);
    };
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
    assert!(!outcome.unknown && !outcome.inventory_changed);
    assert_eq!(
        outcome.conversation.ok_or("offer reply")?.lines,
        ["fixture reply"]
    );
    Ok(true)
}
