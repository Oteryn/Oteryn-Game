//! Select carried sources under the same Character root fence as the stash writer.
use super::*;
use std::collections::BTreeMap;

const MAX_CARRIED_ITEMS: usize = 500;
// Bound each single-asset audit envelope without changing its shape.
const MAX_INPUTS_PER_ENTRY: usize = 20;

pub(super) struct StowInput {
    definition: TypedDefinitionRef,
    count: u32,
    parent: Vec<u8>,
    line: Value,
}

pub(super) async fn select<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    request: &StashRequest,
    native: &NativeGameplayState,
    content: &C,
) -> Result<Result<Vec<Vec<StowInput>>, StashOutcome>, DurabilityError> {
    if !request.row_open {
        return Ok(Err(StashOutcome::Stale));
    }
    let Some(source) = request.source else {
        return Ok(Err(StashOutcome::Stale));
    };
    let world = fence.runtime_scope.world_id();
    let backpack: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(s.item_instance_id) FROM game_item_container_slots s
         JOIN game_item_instances i USING(item_instance_id)
         WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid
         AND i.lifecycle=1 FOR UPDATE OF i",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world.as_bytes().as_slice())
    .fetch_optional(&mut **tx).await?;
    let Some(backpack) = backpack else {
        return Ok(Err(StashOutcome::NoBackpack));
    };
    let rows = sqlx::query(
        "WITH RECURSIVE tree(item,parent,ordinal,depth,path) AS (
          SELECT i.item_instance_id,NULL::uuid,NULL::numeric,0,ARRAY[i.item_instance_id]
          FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid
          UNION ALL
          SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,
                 t.depth+1,t.path||e.item_instance_id
          FROM tree t JOIN LATERAL (
            SELECT item_instance_id,parent_item_instance_id,placement_ordinal
            FROM game_item_container_entries
            WHERE parent_item_instance_id=t.item AND t.depth=0 AND world_id=encode($2,'hex')::uuid
              AND character_id=encode($3,'hex')::uuid
            UNION ALL
            SELECT item_instance_id,parent_item_instance_id,placement_ordinal
            FROM game_item_nested_entries
            WHERE parent_item_instance_id=t.item AND t.depth>0 AND world_id=encode($2,'hex')::uuid
          ) e ON t.depth<8
        ), bounded AS (SELECT * FROM tree LIMIT 502)
        SELECT uuid_send(i.item_instance_id) AS id, i.item_instance_id::text AS item,
               uuid_send(t.parent) AS parent, t.ordinal::text AS ordinal, t.depth,
               encode($4,'hex')::uuid=ANY(t.path) AS inside_selection,
               i.definition_family, i.definition_production_key AS definition_key,
               i.definition_revision_ref AS definition_revision, i.quantity,
               to_jsonb(i) AS before_item,
               NOT EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
               AND NOT EXISTS(SELECT 1 FROM game_item_tiers WHERE item_instance_id=i.item_instance_id)
               AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
               AND NOT EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
               AND NOT EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
               AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
               AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
               AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=i.item_instance_id)
               AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules WHERE item_instance_id=i.item_instance_id)
               AS plain
        FROM bounded t JOIN game_item_instances i ON i.item_instance_id=t.item
        WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1
        ORDER BY i.item_instance_id FOR UPDATE OF i",
    )
    .bind(&backpack)
    .bind(world.as_bytes().as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(source.as_slice())
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > MAX_CARRIED_ITEMS + 1 {
        return Ok(Err(StashOutcome::AmountLimit));
    }
    let Some(selected) = rows.iter().find(|row| row.try_get::<Vec<u8>, _>("id").is_ok_and(|id| id == source)) else {
        return Ok(Err(StashOutcome::Stale));
    };
    let selected_definition = definition(selected)?;
    let contents = matches!(request.intent.action, StashAction::StowContents(_));
    if contents {
        if content.loop_facts(&selected_definition).and_then(|facts| facts.container_capacity).is_none() {
            return Ok(Err(StashOutcome::Unsupported));
        }
    } else if !eligible(selected, native, content)? {
        return Ok(Err(StashOutcome::Unsupported));
    }

    let matching = matches!(request.intent.action, StashAction::StowMatching(_));
    let mut groups = BTreeMap::<_, Vec<_>>::new();
    for row in &rows {
        let id: Vec<u8> = row.try_get("id")?;
        let definition = definition(row)?;
        if if contents {
            id == source || !row.try_get::<bool, _>("inside_selection")?
        } else if matching {
            definition != selected_definition
        } else {
            id != source
        } {
            continue;
        }
        if !eligible(row, native, content)? {
            continue;
        }
        let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let count = match request.intent.action {
            StashAction::StowQuantity { amount, .. } if amount > quantity => {
                return Ok(Err(StashOutcome::Insufficient));
            }
            StashAction::StowQuantity { amount, .. } => amount,
            _ => quantity,
        };
        let parent: Vec<u8> = row.try_get("parent")?;
        let line = json!({
            "item": row.try_get::<String, _>("item")?,
            "count": count,
            "ordinal": row.try_get::<String, _>("ordinal")?,
            "before": row.try_get::<Value, _>("before_item")?,
            "parent": &parent,
            "nested": row.try_get::<i32, _>("depth")? > 1,
        });
        groups.entry((definition.production_key.clone(), definition.revision_ref.clone()))
            .or_default().push(StowInput { definition, count, parent, line });
    }
    let mut batches = Vec::new();
    for inputs in groups.into_values() {
        let mut inputs = inputs.into_iter();
        while !inputs.as_slice().is_empty() {
            batches.push(inputs.by_ref().take(MAX_INPUTS_PER_ENTRY).collect());
        }
    }
    if batches.is_empty() {
        return Ok(Err(StashOutcome::Unsupported));
    }
    Ok(Ok(batches))
}

fn eligible<C: ItemLoopContent>(
    row: &sqlx::postgres::PgRow,
    native: &NativeGameplayState,
    content: &C,
) -> Result<bool, DurabilityError> {
    let definition = definition(row)?;
    let quantity: i64 = row.try_get("quantity")?;
    Ok(row.try_get::<i32,_>("depth")? > 0
        && row.try_get::<bool,_>("plain")?
        && row.try_get::<String,_>("definition_family")? == "Item"
        && stowable(native, &definition)
        && content.loop_facts(&definition).is_some_and(|facts| {
            facts.definition == definition && facts.container_capacity.is_none()
                && stack_maximum(facts.stack).is_ok_and(|(maximum, stackable)| {
                    stackable && (1..=i64::from(maximum)).contains(&quantity)
                })
        }))
}

pub(super) async fn plan(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    inputs: Vec<StowInput>,
) -> Result<Result<Plan, StashOutcome>, DurabilityError> {
    let first = inputs.first().ok_or(DurabilityError::InvalidStoredState)?;
    let definition = first.definition.clone();
    let parent = first.parent.clone();
    let amount = inputs.iter().map(|input| input.count).sum::<u32>();
    let (before, previous) = balance(tx, fence, &definition).await?;
    if let Err(refusal) = check_stow_capacity(tx, fence, before, amount).await? {
        return Ok(Err(refusal));
    }
    let entry = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx).await?;
    Ok(Ok(Plan {
        definition, entry, amount, before, previous, parent,
        lines: inputs.into_iter().map(|input| input.line).collect(),
    }))
}
