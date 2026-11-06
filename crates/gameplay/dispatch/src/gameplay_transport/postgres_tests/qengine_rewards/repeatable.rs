//! Real costed repeats must mint again while exact command replay stays idempotent.
use super::*;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    npc: EntityRef,
    callback: &RewardCallback,
) -> TestResult {
    let input = callback.take.first().ok_or("cost")?;
    let gift = callback.give.as_ref().ok_or("souvenir")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: input.item.clone(),
            count: input.count,
        },
        700,
        true,
    )
    .await?;
    let before = counts(h).await?;
    let (result, _, command) = wire_grant(owner, admitted, npc, callback).await?;
    assert_eq!(
        result,
        NpcIntentDisposition::Ok,
        "a new paid occurrence may repeat"
    );
    let after = counts(h).await?;
    assert_eq!(after.0, before.0 + 1);
    assert_eq!(after.2, before.2 + 1);
    let consumed: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$1")
        .bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(consumed, 2 * i64::from(input.count));
    let gifts: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(&gift.item).fetch_one(&h.pool).await?;
    assert_eq!(gifts, 2 * i64::from(gift.count));
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: callback.keywords[0].clone(),
    })
    .map_err(debug)?;
    let replay =
        super::super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        counts(h).await?,
        after,
        "replay cannot charge or mint again"
    );
    let (refused, _, _) = wire_grant(owner, admitted, npc, callback).await?;
    assert_eq!(
        refused,
        NpcIntentDisposition::Rejected,
        "seven tokens cannot pay twenty"
    );
    assert_eq!(counts(h).await?, after, "unpaid repeat leaves no effects");
    Ok(())
}
