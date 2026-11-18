//! Resolve a bounded carried requirement across real instances before any BURN.
use super::*;

pub(super) async fn resolve(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    required: &oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount,
) -> Result<Option<Vec<Input>>, DurabilityError> {
    // Each matched row contributes at least one unit. The requested quantity
    // bounds the number of objects needed to distinguish success from absence.
    let rows = sqlx::query("SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,c.location,c.kind,i.quantity FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.quantity>0 AND i.definition_family='Item' AND i.definition_production_key=$3 ORDER BY i.item_instance_id LIMIT $4 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(&required.item)
        .bind(i64::from(required.count))
        .fetch_all(&mut **tx).await?;
    let mut remaining = required.count;
    let mut inputs = Vec::new();
    for row in rows {
        let quantity: i64 = row.try_get("quantity")?;
        let count = u16::try_from(quantity.min(i64::from(remaining)))
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        inputs.push(Input {
            item: row.try_get("item")?,
            before: row.try_get("before_item")?,
            location: row.try_get("location")?,
            kind: row.try_get("kind")?,
            quantity,
            count,
            replacement: None,
        });
        remaining -= count;
        if remaining == 0 {
            break;
        }
    }
    Ok((remaining == 0).then_some(inputs))
}
