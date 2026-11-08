//! Retain source merchant and postage checks under the existing Character lock.
use super::*;
use oteryn_gameplay_content::content::npc_catalogue::service;

pub(super) async fn permitted(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    request: &NpcTradeRequest,
) -> std::result::Result<bool, sqlx::Error> {
    if let Some((quest, track, value)) = service::merchant_access(&request.cause.npc_key) {
        let stored: Option<i64> = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND track_key=$3",
        )
        .bind(character.as_bytes().as_slice()).bind(quest).bind(track)
        .fetch_optional(&mut ***tx).await?;
        if stored != Some(value) { return Ok(false); }
    }
    if request.cause.side != NpcTradeSide::Buy
        || service::postage_price(&request.cause.npc_key, &request.item.definition.production_key, false).is_none()
    { return Ok(true); }
    let rank: Option<i64> = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.the_postman_missions_quest' AND track_key=$2",
    )
    .bind(character.as_bytes().as_slice()).bind(service::POSTMAN_RANK_TRACK)
    .fetch_optional(&mut ***tx).await?;
    Ok(request.offer_count == 1 && service::postage_price(
        &request.cause.npc_key, &request.item.definition.production_key,
        rank.is_some_and(|rank| rank >= 3),
    ) == Some(request.unit_price))
}
