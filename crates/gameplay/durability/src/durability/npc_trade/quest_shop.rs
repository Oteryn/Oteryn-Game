//! Source shop requirements are reread under the existing Character root lock.
use super::*;
use oteryn_gameplay_content::content::{
    ProjectV2DefinitionRef, ProjectV2Family, ProjectV2ServiceOfferDirection,
    npc_catalogue::quest_shops,
};

async fn track_value(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    quest: &str,
    track: &str,
) -> std::result::Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks \
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND track_key=$3",
    )
    .bind(character.as_bytes().as_slice())
    .bind(quest)
    .bind(track)
    .fetch_optional(&mut ***tx)
    .await
}

pub(super) async fn permitted(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    request: &NpcTradeRequest,
) -> std::result::Result<bool, sqlx::Error> {
    if let Some((quest, track)) = quest_shops::ordinary_requirement(&request.cause.npc_key)
        && track_value(tx, character, quest, track).await? != Some(quest_shops::ORDINARY_UNLOCK_VALUE)
    {
        return Ok(false);
    }
    let direction = match request.cause.side {
        NpcTradeSide::Buy => ProjectV2ServiceOfferDirection::SellToPlayer,
        // These exchanges have their own pinned source and custody guards.
        NpcTradeSide::Exchange => {
            return Ok(matches!(
                request.cause.npc_key.as_str(),
                "oteryn:npc.al_dee"
                    | "oteryn:npc.seymour"
                    | sinatuki_exchange::NPC
                    | "oteryn:npc.uzgod"
            ));
        }
        NpcTradeSide::Sell => ProjectV2ServiceOfferDirection::BuyFromPlayer,
    };
    let item = ProjectV2DefinitionRef {
        family: ProjectV2Family::Item,
        key: request.item.definition.production_key.clone(),
        revision: request.item.definition.revision_ref.clone(),
    };
    let offer = match quest_shops::shops().and_then(|shops| {
        quest_shops::required(
            shops,
            &request.cause.npc_key,
            &item,
            direction,
            request.unit_price,
            request.offer_count,
        )
    }) {
        Ok(None) => return Ok(true),
        Ok(Some(offer)) => offer,
        Err(_) => return Ok(false),
    };
    for requirement in &offer.requirements {
        let value = track_value(tx, character, &requirement.quest, &requirement.track).await?;
        if !(requirement.minimum..=requirement.maximum)
            .contains(&value.unwrap_or(requirement.initial))
        {
            return Ok(false);
        }
    }
    Ok(true)
}
