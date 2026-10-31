use super::*;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTradeIntent, NpcTradeSide};

async fn buy_intent(
    owner: &ComposedFreshAdmission<'_, '_, '_>, npc: EntityRef,
    points: i64, item: &str,
) -> TestResult<NpcTradeIntent> {
    let book = owner.states.chat.npc_talk().ok_or("book")?;
    let entry = book.get(quest::NPC).ok_or("Grizzly")?;
    let native = owner.active_generation.and_then(|active| active.native_gameplay()).ok_or("native shop profiles")?;
    let eligible: Vec<_> = entry.offers().iter().filter(|offer|
        crate::content::npc_catalogue::grizzly_shop::visible(quest::NPC, offer, Some(points))
            && crate::gameplay_transport::item_ref_admission::generation_item_facts(native,
                &crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(), production_key: offer.item.key.clone(), revision_ref: offer.item.revision.clone(),
                }).is_ok_and(|facts| facts.weight.is_some()
                    && !matches!(facts.stack, crate::durability::item_transfer::ItemStackClass::Unknown)))
        .cloned().collect();
    let runtime = owner.runtime.lock().await;
    let index = crate::gameplay_transport::item_ref_admission::pinned_index(
        &runtime, owner.active_generation).ok_or("item index")?;
    let (offer_index, (shown, _)) = oteryn_gameplay_npc::npc_command::projected(&eligible, index)
        .enumerate().find(|(_, (_, source))| source.item.key == item).ok_or("source shop offer")?;
    assert_eq!(shown.side, NpcTradeSide::Buy);
    Ok(NpcTradeIntent {
        npc_actor: npc, catalogue_revision: book.catalogue_revision(),
        offer_index: u32::try_from(offer_index)?, side: shown.side, quantity: 1,
        expected_unit_price: shown.unit_price,
    })
}

pub(super) async fn rank_shop(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    npc: EntityRef, h: &Harness,
) -> TestResult {
    super::super::item_look::seed_main_backpack(h, &[("oteryn:item.tibia.i3043", 4)]).await?;
    let sling = buy_intent(owner, npc, 20, "oteryn:item.tibia.i5907").await?;
    seed(h, &[(quest::POINTS_STORAGE, 19)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let refused = wire::trade(owner, admitted, h, 3, &sling, None, false).await?;
    assert_eq!(refused, vec![NpcIntentDisposition::Stale], "rank-19 window omits slingshot");

    seed(h, &[(quest::POINTS_STORAGE, 20)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let refused = wire::trade(owner, admitted, h, 6, &sling, Some(19), false).await?;
    assert_eq!(refused, vec![NpcIntentDisposition::Rejected],
        "rank-20 display cache cannot bypass the transaction's native rank-19 state");
    let trades: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades").fetch_one(&h.pool).await?;
    assert_eq!(trades, 0, "refusals never burn coins or mint a trade");

    seed(h, &[(quest::POINTS_STORAGE, 20)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    assert_eq!(wire::trade(owner, admitted, h, 9, &sling, None, true).await?,
        vec![NpcIntentDisposition::Ok, NpcIntentDisposition::Ok]);
    let backpack = buy_intent(owner, npc, 70, "oteryn:item.tibia.i9601").await?;
    seed(h, &[(quest::POINTS_STORAGE, 69)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    assert_eq!(wire::trade(owner, admitted, h, 12, &backpack, None, false).await?,
        vec![NpcIntentDisposition::Stale]);
    seed(h, &[(quest::POINTS_STORAGE, 70)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    assert_eq!(wire::trade(owner, admitted, h, 15, &backpack, None, true).await?,
        vec![NpcIntentDisposition::Ok, NpcIntentDisposition::Ok]);

    let (trades, slings, bags): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_npc_trades), \
         (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i5907'), \
         (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i9601')",
    ).fetch_one(&h.pool).await?;
    assert_eq!((trades, slings, bags), (2, 1, 1));
    Ok(())
}
