//! Source shop requirements are reread under the existing Character root lock.
use super::*;
use oteryn_gameplay_content::content::{
    ProjectV2DefinitionRef, ProjectV2Family, ProjectV2ServiceOfferDirection,
    npc_catalogue::quest_shops,
};

pub(super) async fn permitted(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    request: &NpcTradeRequest,
) -> std::result::Result<bool, sqlx::Error> {
    let direction = match request.cause.side {
        NpcTradeSide::Buy => ProjectV2ServiceOfferDirection::SellToPlayer,
        // These exchanges have their own pinned source and custody guards.
        NpcTradeSide::Exchange => {
            return Ok(matches!(
                request.cause.npc_key.as_str(),
                "oteryn:npc.al_dee" | "oteryn:npc.seymour" | "oteryn:npc.uzgod"
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
        let value:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND track_key=$3")
            .bind(character.as_bytes().as_slice()).bind(&requirement.quest).bind(&requirement.track).fetch_optional(&mut ***tx).await?;
        if !(requirement.minimum..=requirement.maximum)
            .contains(&value.unwrap_or(requirement.initial))
        {
            return Ok(false);
        }
    }
    Ok(true)
}
