//! Rank-gated Grizzly offers use the existing NPC item transaction and Character lock.
use super::*;
use oteryn_gameplay_content::content::{
    ProjectV2ServiceOfferDirection, npc_catalogue::grizzly_shop,
};
use oteryn_gameplay_quests::quest::grizzly_adams as quest;

pub(super) async fn permitted(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    request: &NpcTradeRequest,
) -> std::result::Result<bool, sqlx::Error> {
    let direction = match request.cause.side {
        NpcTradeSide::Buy => ProjectV2ServiceOfferDirection::SellToPlayer,
        NpcTradeSide::Sell => ProjectV2ServiceOfferDirection::BuyFromPlayer,
    };
    let minimum = match grizzly_shop::required_points(
        &request.cause.npc_key,
        &request.item.definition.production_key,
        direction,
        request.unit_price,
        request.offer_count,
    ) {
        Ok(None) => return Ok(true),
        Ok(Some(minimum)) => minimum,
        Err(_) => return Ok(false),
    };
    // character_item_fence_is_current has already locked this Character root. The quest
    // owner cannot change its points until this item transaction commits or rolls back.
    let points: Option<i64> = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid \
         AND quest_key=$2 AND track_key=$3",
    )
    .bind(character.as_bytes().as_slice())
    .bind(quest::QUEST)
    .bind(quest::storage_track(quest::POINTS_STORAGE))
    .fetch_optional(&mut ***tx)
    .await?;
    Ok(points.is_some_and(|points| points >= minimum))
}
