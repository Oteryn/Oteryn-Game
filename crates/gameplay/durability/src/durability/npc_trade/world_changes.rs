//! World-change shops use the ordinary trade owner and the same native NPC context.
use super::super::quest_state::world_change_npcs::NativeWorldChangeNpc;
use super::*;
use oteryn_gameplay_content::content::{
    ProjectV2ServiceOfferDirection, npc_catalogue::world_change_shop,
};

/// Called after the existing current item fence, before coins/items are touched.
/// Existing vendors pass unchanged; a world-change NPC requires its native source.
pub(super) async fn permitted(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &NpcTradeRequest,
    source: Option<&NativeWorldChangeNpc>,
) -> std::result::Result<bool, DurabilityError> {
    let direction = match request.cause.side {
        NpcTradeSide::Buy => ProjectV2ServiceOfferDirection::SellToPlayer,
        NpcTradeSide::Sell => ProjectV2ServiceOfferDirection::BuyFromPlayer,
    };
    let change = match world_change_shop::required_change(
        &request.cause.npc_key,
        &request.item.definition.production_key,
        direction,
        request.unit_price,
        request.offer_count,
    ) {
        Ok(None) => return Ok(source.is_none()),
        Ok(Some(change)) => change,
        Err(_) => return Ok(false),
    };
    let Some(source) = source else {
        return Ok(false);
    };
    if source.npc_key() != request.cause.npc_key
        || !source.matches_item(fence, request.command)
        || !source.source_is_current(tx).await?
    {
        return Ok(false);
    }
    source.selected_npc(tx, change).await
}
