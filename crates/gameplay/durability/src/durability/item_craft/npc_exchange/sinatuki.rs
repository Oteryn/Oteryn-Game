//! Wiki-pinned Sinatuki: eat a whole first stack, accumulate 99 mixed fish,
//! then accept a further stack for five shards and Jamjam, once per Character.
//! Progress is derived from immutable craft receipts under the Character root
//! lock already held by the caller; no parallel storage or unfenced write exists.
use super::*;

pub(super) async fn conversion(
    tx: &mut super::super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
) -> Result<Option<oteryn_gameplay_items::item_craft::CookingConversion>, DurabilityError> {
    if definition.family != "Item"
        || definition.revision_ref != "definition-r1"
        || !matches!(
            definition.production_key.as_str(),
            "oteryn:item.tibia.i7158" | "oteryn:item.tibia.i7159"
        )
    {
        return Ok(None);
    }
    let access: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.the_ice_islands_quest' AND track_key='oteryn:quest-progress/quest/u8_0/barbarian_test/questline' AND value>=3) \
         AND EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.the_ice_islands_quest' AND track_key='oteryn:quest-progress/quest/u8_0/the_ice_islands/questline' AND value>=5)",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    if !access {
        return Ok(None);
    }
    // Each partial consumes at least one fish, so at most 99 partial receipts
    // and one final receipt can exist. The SQL admission trigger enforces this.
    let history: Vec<(Value, Value)> = sqlx::query_as(
        "SELECT before_item,outputs FROM game_item_craft_records \
         WHERE character_id=encode($1,'hex')::uuid AND recipe='oteryn:npc-quest-exchange.sinatuki' \
         ORDER BY transaction_id LIMIT 101",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_all(&mut ***tx)
    .await?;
    let mut eaten = 0_i64;
    for (before, outputs) in history {
        if !outputs.as_array().is_some_and(Vec::is_empty) {
            return Ok(None);
        }
        eaten = eaten
            .checked_add(
                before
                    .get("quantity")
                    .and_then(Value::as_i64)
                    .filter(|count| (1..=100).contains(count))
                    .ok_or(DurabilityError::InvalidStoredState)?,
            )
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    let outputs = if eaten >= 99 {
        let Some(output) = native
            .item_policy_for_source_id(7290)
            .map(|item| item.record())
            .filter(|record| record.admission.materializable)
        else {
            return Ok(None);
        };
        vec![(
            TypedDefinitionRef {
                family: output.production_definition.family.clone(),
                production_key: output.production_definition.production_key.clone(),
                revision_ref: output.production_definition.revision_ref.clone(),
            },
            5,
        )]
    } else {
        Vec::new()
    };
    Ok(Some(oteryn_gameplay_items::item_craft::CookingConversion {
        recipe: "oteryn:npc-quest-exchange.sinatuki".into(),
        outputs,
    }))
}
