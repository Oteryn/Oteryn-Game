//! Resolve a complete native repair budget before the existing proof/burn owner writes.
use super::*;

pub(super) async fn collect(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    required: &oteryn_gameplay_quests::quest::world_callbacks::ItemOnTarget,
) -> Result<Option<Vec<Input>>, DurabilityError> {
    if required.reusable
        || required.replacement.is_some()
        || !required.alternatives.is_empty()
        || !(1..=64).contains(&required.count)
    {
        return Ok(None);
    }
    // Positive quantities mean at most64 instances can satisfy any admitted budget.
    // Additional owned instances are irrelevant; their existence is not an overflow.
    let rows = sqlx::query(
        "SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,\
         c.location,c.kind,i.quantity FROM \
         game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c \
         JOIN game_item_instances i ON i.item_instance_id=c.item \
         WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 \
         AND i.definition_family='Item' AND i.definition_production_key=$3 \
         AND i.quantity>0 ORDER BY i.item_instance_id LIMIT 64 FOR UPDATE OF i",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(&required.item)
    .fetch_all(&mut **tx)
    .await?;
    let mut needed = i64::from(required.count);
    let mut inputs = Vec::new();
    for row in rows {
        if needed == 0 {
            break;
        }
        let quantity: i64 = row.try_get("quantity")?;
        let count =
            u16::try_from(needed.min(quantity)).map_err(|_| DurabilityError::InvalidStoredState)?;
        let input = Input {
            item: row.try_get("item")?,
            before: row.try_get("before_item")?,
            location: row.try_get("location")?,
            kind: row.try_get("kind")?,
            quantity,
            count,
            replacement: None,
        };
        if input.kind == "ground" {
            return Ok(None);
        }
        lock_location(tx, &input).await?;
        let plain: bool = sqlx::query_scalar(
            "SELECT NOT (EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=$1::uuid) \
             OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=$1::uuid) \
             OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=$1::uuid) \
             OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=$1::uuid) \
             OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=$1::uuid) \
             OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=$1::uuid))",
        ).bind(&input.item).fetch_one(&mut **tx).await?;
        if !plain {
            return Ok(None);
        }
        needed -= i64::from(count);
        inputs.push(input);
    }
    Ok((needed == 0).then_some(inputs))
}
