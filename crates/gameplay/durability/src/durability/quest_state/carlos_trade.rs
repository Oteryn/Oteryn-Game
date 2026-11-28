//! The first Carlos food sale advances the existing tutorial in the trade transaction.
use super::{QuestCause, QuestTransitionRequest, persist};
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use crate::durability::npc_trade::{NpcTradeRequest, NpcTradeSide};
use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestTrackChange, QuestTransition};
use sqlx::Row;

pub const NPC: &str = "oteryn:npc.carlos";
pub const GREETING: &str =
    "oteryn:quest-progress/quest/u8_2/the_beginning_quest/carlos_npc_greet_storage";
const QUEST: &str = "oteryn:quest.the_beginning_quest";
const TRANSITION: &str = "oteryn:quest-transition/beginning/carlos-food-sale";

pub fn is_food_sale(request: &NpcTradeRequest) -> bool {
    let item = &request.item.definition;
    request.cause.npc_key == NPC
        && request.cause.side == NpcTradeSide::Sell
        && request.unit_price == 2
        && request.offer_count == 1
        && item.family == "Item"
        && item.revision_ref == "definition-r1"
        && matches!(item.production_key.as_str(), "oteryn:item.tibia.i3577" | "oteryn:item.tibia.i3582")
}

/// The trade writer holds the current Character item fence and has persisted the
/// sale. An error rolls both it and the quest successor back.
pub(in crate::durability) async fn advance(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    sale: &NpcTradeRequest,
) -> Result<(), DurabilityError> {
    let state = sqlx::query(
        "SELECT c.character_revision::text, s.pinned_content_revision, s.definition_hash
         FROM game_character_roots c
         JOIN game_character_quest_states s USING(character_id)
         JOIN game_character_quest_tracks t USING(character_id,quest_key)
         WHERE c.character_id=encode($1,'hex')::uuid AND s.quest_key=$2
           AND t.track_key=$3 AND t.value=6 AND s.completed_character_revision IS NULL",
    )
    .bind(character.as_bytes().as_slice())
    .bind(QUEST)
    .bind(GREETING)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(state) = state else { return Ok(()) };
    let revision: String = state.try_get("character_revision")?;
    let pin: String = state.try_get("pinned_content_revision")?;
    let hash: Vec<u8> = state.try_get("definition_hash")?;
    let request = QuestTransitionRequest {
        transition_key: TRANSITION.to_owned(),
        cause: QuestCause::Command(sale.command),
    };
    let transition = QuestTransition {
        key: request.transition_key.clone(),
        quest: QUEST.to_owned(),
        effects: vec![QuestEffect {
            track: GREETING.to_owned(),
            from: QuestComparison::Eq(6),
            effect: QuestEffectKind::Set(7),
        }],
        completes: false,
        experience: None,
        world_change: None,
    };
    persist::persist(tx, persist::QuestWrite {
        character,
        root_revision: revision.parse().map_err(|_| DurabilityError::InvalidStoredState)?,
        request: &request,
        transition: &transition,
        pin: &pin,
        hash: hash.as_slice().try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
        changes: &[QuestTrackChange { track: GREETING.to_owned(), before: 6, after: 7 }],
        binding: &super::request_binding(character, &request),
        state_exists: true,
    }).await?;
    Ok(())
}
