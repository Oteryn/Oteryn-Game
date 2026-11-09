//! Source offer level refusals use real Character progression and the common topic.
use super::fixture::Harness;
use super::{ComposedFreshAdmission, Dependencies, TestResult};
use crate::gameplay_transport::connection::UseCommand;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

#[test]
fn engnpc_offer_level_refusal_uses_character_progression_and_resets_topic() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .name("engnpc-offer-levels".into())
        .stack_size(256 << 20)
        .spawn(|| super::npc_paid_travel::offer_level_scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "offer level thread failed")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED engnpc_offer_level_refusal_uses_character_progression_and_resets_topic duration={:?}",
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
    let fence = owner
        .current_quest_fence(dependencies.admitted.game_session_id)
        .await
        .map_err(|_| "level fence")?
        .ok_or("level session")?
        .item_fence();
    let level = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(|_| "progression read")?
        .ok_or("progression row")?
        .level;
    assert!(level > 0, "the fixture must exercise a real positive level");
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    let offer = |topic, next_topic| {
        serde_json::json!({
            "npc":"oteryn:npc.captain_bluebear", "keywords":["fixture level"],
            "topic":topic,"next_topic":next_topic,"storage":[],"reply":["accepted"],
        })
    };
    apply(
        owner,
        dependencies,
        npc,
        9911,
        offer(0, 7),
        Some("accepted"),
    )
    .await?;
    let mut refused = offer(7, 9);
    refused["minimum_level"] = (level + 1).into();
    refused["failure_reply"] = serde_json::json!(["level refused"]);
    apply(
        owner,
        dependencies,
        npc,
        9912,
        refused.clone(),
        Some("level refused"),
    )
    .await?;
    apply(owner, dependencies, npc, 9913, offer(7, 0), None).await?;
    let mut boundary = offer(0, 7);
    boundary["minimum_level"] = level.into();
    boundary["failure_reply"] = serde_json::json!(["level refused"]);
    apply(owner, dependencies, npc, 9914, boundary, Some("accepted")).await?;
    refused["failure_reply"] = serde_json::json!([]);
    apply(owner, dependencies, npc, 9915, refused, None).await?;
    apply(
        owner,
        dependencies,
        npc,
        9916,
        offer(7, 0),
        Some("accepted"),
    )
    .await?;
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(before, after, "read-only refusal must not mint a receipt");
    eprintln!("ENGNPC_OFFER_LEVEL observed_level={level} boundary=accepted above=refused");
    Ok(())
}

async fn apply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
    npc: EntityRef,
    command_id: u64,
    offer: serde_json::Value,
    expected: Option<&str>,
) -> TestResult {
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
            text: "fixture level".into(),
        },
        &source,
    )
    .await;
    let Some(expected) = expected else {
        assert!(
            outcome.is_none(),
            "ineligible offer must not publish or change topic"
        );
        return Ok(());
    };
    let outcome = outcome.ok_or("expected source offer reply was not selected")?;
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
    assert!(!outcome.unknown && !outcome.inventory_changed);
    assert_eq!(outcome.conversation.ok_or("offer reply")?.lines, [expected]);
    Ok(())
}
