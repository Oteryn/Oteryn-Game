//! Native service access is checked inside the existing fenced Character item transaction.
use super::*;
use oteryn_gameplay_content::content::{
    ProjectV2DefinitionRef, ProjectV2Family, ProjectV2ServiceOfferDirection,
    npc_catalogue::quest_access,
};

pub(super) async fn permitted(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    request: &NpcTradeRequest,
) -> std::result::Result<bool, sqlx::Error> {
    match quest_access::rule(&request.cause.npc_key) {
        Ok(None) => return Ok(true),
        Ok(Some(_)) => {}
        Err(_) => return Ok(false),
    }
    if request.item.definition.family != "Item" {
        return Ok(false);
    }
    let direction = match request.cause.side {
        NpcTradeSide::Buy => ProjectV2ServiceOfferDirection::SellToPlayer,
        NpcTradeSide::Exchange => return Ok(false),
        NpcTradeSide::Sell => ProjectV2ServiceOfferDirection::BuyFromPlayer,
    };
    let definition = ProjectV2DefinitionRef {
        family: ProjectV2Family::Item,
        key: request.item.definition.production_key.clone(),
        revision: request.item.definition.revision_ref.clone(),
    };
    let rule = match quest_access::required(
        &request.cause.npc_key,
        &definition,
        direction,
        request.unit_price,
        request.offer_count,
    ) {
        Ok(Some(rule)) => rule,
        _ => return Ok(false),
    };
    // character_item_fence_is_current already locked this Character root. Neither
    // the quest state nor its tracks can change until this item transaction ends.
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_character_quest_states state \
         JOIN game_character_quest_tracks track ON track.character_id=state.character_id \
         AND track.quest_key=state.quest_key \
         WHERE state.character_id=encode($1,'hex')::uuid AND state.quest_key=$2 \
         AND state.completed_character_revision IS NOT NULL \
         AND track.track_key=$3 AND track.value BETWEEN $4 AND $5)",
    )
    .bind(character.as_bytes().as_slice())
    .bind(&rule.quest)
    .bind(&rule.track)
    .bind(rule.minimum)
    .bind(rule.maximum)
    .fetch_one(&mut ***tx)
    .await
}
