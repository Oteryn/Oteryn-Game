//! Resolve all carried inputs and server-owned per-input rolls under the quest fence.
use super::*;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::reward_callbacks::{ItemAmount, RewardCallback};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct BulkDraw {
    input_count: u16,
    rolls: Vec<i32>,
    output_count: u32,
}

pub(super) async fn resolve(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    callback: &RewardCallback,
    native: &NativeGameplayState,
) -> std::result::Result<Option<(RewardCallback, BulkDraw)>, DurabilityError> {
    let config = callback
        .bulk
        .as_ref()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let rows: Vec<i64> = sqlx::query_scalar("SELECT i.quantity FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$3 ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(&config.item).fetch_all(&mut **tx).await?;
    if rows.is_empty() || rows.len() > 500 {
        return Ok(None);
    }
    let count = rows
        .iter()
        .try_fold(0_i64, |sum, quantity| sum.checked_add(*quantity))
        .and_then(|sum| u16::try_from(sum).ok())
        .filter(|count| *count > 0);
    let Some(count) = count else {
        return Ok(None);
    };
    let id = config
        .output
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|tail| tail.parse::<u32>().ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let policy = native
        .item_policy_for_source_id(id)
        .ok_or(DurabilityError::InvalidStoredState)?
        .record();
    let facts = oteryn_gameplay_items::item_ref_admission::generation_item_facts(
        native,
        &TypedDefinitionRef {
            family: policy.production_definition.family.clone(),
            production_key: config.output.clone(),
            revision_ref: policy.production_definition.revision_ref.clone(),
        },
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let (maximum, _) =
        stack_maximum(facts.stack).map_err(|_| DurabilityError::InvalidStoredState)?;
    let base = u32::from(count) * u32::from(config.base);
    if base > maximum * 20 {
        return Ok(None);
    }
    // The database function owns randomness. The player supplies neither a seed nor rolls.
    let rolls: Vec<i32> = sqlx::query_scalar("SELECT game_quest_reward_rolls($1)")
        .bind(i32::from(count))
        .fetch_one(&mut **tx)
        .await?;
    if rolls.len() != usize::from(count) || rolls.iter().any(|roll| !(1..=100).contains(roll)) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let bonuses = u32::try_from(
        rolls
            .iter()
            .filter(|roll| **roll <= i32::from(config.chance_percent))
            .count(),
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let total = base + bonuses * u32::from(config.bonus);
    if total > maximum * 20 {
        return Ok(None);
    }
    let mut resolved = callback.clone();
    resolved.take[0].count = count;
    let give = resolved
        .give
        .as_mut()
        .ok_or(DurabilityError::InvalidStoredState)?;
    give.count =
        u16::try_from(total.min(maximum)).map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut remaining = total - u32::from(give.count);
    while remaining > 0 {
        let quantity = remaining.min(maximum);
        give.backpack_outputs.push(ItemAmount {
            item: config.output.clone(),
            fluid_subtype: None,
            count: u16::try_from(quantity).map_err(|_| DurabilityError::InvalidStoredState)?,
        });
        remaining -= quantity;
    }
    Ok(Some((
        resolved,
        BulkDraw {
            input_count: count,
            rolls,
            output_count: total,
        },
    )))
}
