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

// Each shard uses one admitted player and the production dialogue/transaction
// owner to buy its complete section of Jorge's pinned 37-souvenir menu.
pub(super) async fn menu(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    npc: EntityRef,
    part: usize,
) -> TestResult {
    let callbacks: Vec<_> = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .filter(|c| c.npc == "oteryn:npc.jorge")
        .collect();
    assert_eq!(
        callbacks.len(),
        37,
        "all pinned donor souvenirs must be playable"
    );
    for (i, callback) in callbacks.iter().enumerate().skip(part * 13).take(13) {
        let input = callback.take.first().ok_or("token cost")?;
        assert_eq!(input.count, 20);
        let gift = callback.give.as_ref().ok_or("souvenir")?;
        let before: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
            .bind(&gift.item).fetch_one(&h.pool).await?;
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
                count: 20,
            },
            1000 + u64::try_from(i)?,
            true,
        )
        .await?;
        let carried: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i WHERE i.lifecycle=1 AND i.definition_production_key=$1 AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)")
            .bind(&input.item).fetch_one(&h.pool).await?;
        assert!(
            carried >= 20,
            "the fixture must actually carry its payment: {carried}"
        );
        let receipts = counts(h).await?;
        let (result, _, command) = wire_grant(owner, admitted, npc, callback).await?;
        assert_eq!(result, NpcIntentDisposition::Ok, "{}", callback.key);
        let after = counts(h).await?;
        assert_eq!(after.0, receipts.0 + 1);
        assert_eq!(after.2, receipts.2 + 1);
        let quantity: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
            .bind(&gift.item).fetch_one(&h.pool).await?;
        assert_eq!(quantity, before + 1, "{}", gift.item);
        let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc,
            text: callback.keywords[0].clone(),
        })
        .map_err(debug)?;
        let replay =
            super::super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay)
                .map_err(debug)?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(counts(h).await?, after, "replay must not pay or mint again");
        let (unpaid, _, _) = wire_grant(owner, admitted, npc, callback).await?;
        assert_eq!(unpaid, NpcIntentDisposition::Rejected);
        assert_eq!(counts(h).await?, after);
    }
    Ok(())
}
