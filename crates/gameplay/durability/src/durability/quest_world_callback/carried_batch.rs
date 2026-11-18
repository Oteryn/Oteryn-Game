//! Resolve a bounded carried requirement across real instances before any BURN.
use super::*;

pub(super) async fn resolve(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    required: &oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount,
    first_only: bool,
) -> Result<Option<Vec<Input>>, DurabilityError> {
    let selected = if first_only {
        let Some(item) = first_definition(tx, fence, &required.item).await? else {
            return Ok(None);
        };
        Some(item)
    } else {
        None
    };
    let rows = sqlx::query("SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,c.location,c.kind,i.quantity FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$5::text::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.quantity>0 AND i.definition_family='Item' AND i.definition_production_key=$3 AND ($4::smallint IS NULL OR EXISTS (SELECT 1 FROM game_item_fluid_states f WHERE f.item_instance_id=i.item_instance_id AND f.fluid_subtype=$4)) ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(&required.item)
        .bind(required.fluid_subtype.map(i16::from))
        .bind(selected)
        .fetch_all(&mut **tx).await?;
    if rows.len() > 500 {
        return Ok(None);
    }
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

async fn first_definition(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    definition: &str,
) -> Result<Option<String>, DurabilityError> {
    // Crystal 201189f8 movements_marziel_blood.lua:22-25 calls getItemById without a
    // subtype. Game::findItemOfType visits worn slots, then bags breadth-first.
    // The custody reader owns the tree limits. Reverse ordinals are Bag display order.
    Ok(sqlx::query_scalar(
        "WITH RECURSIVE carried AS MATERIALIZED (
           SELECT * FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid)
         ), display_tree(item, depth, display_path) AS (
           SELECT item,0,ARRAY[COALESCE((location->>'slot')::numeric,3::numeric)]
             FROM carried WHERE kind IN ('backpack','equipment')
           UNION ALL
           SELECT c.item,p.depth+1,p.display_path || -((c.location->>'placement_ordinal')::numeric)
             FROM display_tree p JOIN carried c ON (c.location->>'parent_item_instance_id')::uuid=p.item
             WHERE p.depth<8 AND c.kind IN ('container','nested')
         ) SELECT i.item_instance_id::text FROM display_tree d
           JOIN game_item_instances i ON i.item_instance_id=d.item
          WHERE i.lifecycle=1 AND i.quantity>0 AND i.definition_family='Item'\
            AND i.definition_production_key=$3 ORDER BY d.depth,d.display_path LIMIT 1",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(definition)
    .fetch_optional(&mut **tx)
    .await?)
}
