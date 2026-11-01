//! Delivery display uses the same custody families as the delivery participant.
use super::{TaskBoardCatalogue, TaskBoardView};
use crate::durability::DurabilityError;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use sqlx::PgConnection;
use std::collections::BTreeMap;

/// The caller owns the current Character root lock. Aggregate in PostgreSQL and
/// return at most nine definitions, never a clipped list of Item instances.
/// Crystal protocolgame.cpp:6210-6222 adds three uint32 counts for unfinished
/// rows. Modulo here preserves that informative display, not a currency limit;
/// the custody participant still consumes the actual quantities independently.
pub(super) async fn refresh(
    tx: &mut PgConnection,
    fence: CurrentCharacterGameplayFence,
    catalogue: &TaskBoardCatalogue,
    view: &mut TaskBoardView,
) -> Result<(), DurabilityError> {
    view.delivery_available.clear();
    let Some(weekly) = &view.data.weekly else {
        return Ok(());
    };
    if weekly.deliveries.len() > 9 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut requested = BTreeMap::new();
    for delivery in &weekly.deliveries {
        view.delivery_available.insert(delivery.item_id, 0);
        if let Some(item) = catalogue.items.get(&delivery.item_id) {
            requested.insert(delivery.item_id, item.key.clone());
        }
    }
    if requested.is_empty() {
        return Ok(());
    }
    let keys: Vec<String> = requested.values().cloned().collect();
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "WITH quantities AS ( \
           SELECT i.definition_production_key AS item_key,i.quantity \
             FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c \
             JOIN game_item_instances i ON i.item_instance_id=c.item \
            WHERE i.lifecycle=1 AND i.definition_family='Item' \
              AND i.definition_production_key=ANY($3::text[]) \
           UNION ALL \
           SELECT definition_key,quantity FROM game_character_stash_balances \
            WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid \
              AND definition_key=ANY($3::text[]) AND quantity>0 \
           UNION ALL \
           SELECT i.definition_production_key,i.quantity \
             FROM game_task_board_depot_items(encode($1,'hex')::uuid,encode($2,'hex')::uuid,NULL,NULL,NULL) d \
             JOIN game_item_instances i ON i.item_instance_id=d.item \
            WHERE i.definition_production_key=ANY($3::text[]) \
         ) SELECT item_key,mod(sum(quantity),4294967296)::bigint \
             FROM quantities GROUP BY item_key",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(keys)
    .fetch_all(tx)
    .await?;
    if rows.len() > 9 {
        return Err(DurabilityError::InvalidStoredState);
    }
    for (key, count) in rows {
        let count = u32::try_from(count).map_err(|_| DurabilityError::InvalidStoredState)?;
        for (&item, expected) in &requested {
            if expected == &key {
                view.delivery_available.insert(item, count);
            }
        }
    }
    Ok(())
}
