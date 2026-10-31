//! Lock and revalidate the consumed source ancestry under the Character root lock.

use super::super::item_loop_move::ITEM_LOOP_DEPTH_MAX;
use super::*;

/// The source-to-main chain, bounded including the insertion edge (BAGS0-RL-01).
async fn use_path(connection: &mut PgConnection, source: &[u8; 16]) -> Result<Vec<[u8; 16]>> {
    let rows: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE path(item, depth) AS ( \
           SELECT encode($1,'hex')::uuid, 0 \
           UNION ALL \
           SELECT e.parent_item_instance_id, p.depth + 1 FROM path p JOIN ( \
             SELECT item_instance_id, parent_item_instance_id FROM game_item_container_entries \
             UNION ALL \
             SELECT item_instance_id, parent_item_instance_id FROM game_item_nested_entries \
           ) e ON e.item_instance_id = p.item WHERE p.depth < $2 \
         ) SELECT item::text FROM path ORDER BY depth LIMIT 10",
    )
    .bind(source.as_slice())
    .bind(ITEM_LOOP_DEPTH_MAX + 1)
    .fetch_all(connection)
    .await?;
    rows.iter()
        .map(|id| uuid_text(id).map_err(Into::into))
        .collect()
}

pub(super) async fn lock_use_path(
    connection: &mut PgConnection,
    holder: &[u8; 16],
    backpack: &[u8; 16],
    world: &[u8; 16],
    request: &ItemUseRequest,
) -> Result<()> {
    let tool = match request.kind {
        ItemUseKind::PickRefine { tool, .. } => Some(tool),
        _ => None,
    };
    let flask = match request.flask {
        ItemUseFlaskPlan::Stack {
            item_instance_id, ..
        } => Some(item_instance_id),
        _ => None,
    };
    lock_carried_path(
        connection,
        holder,
        backpack,
        world,
        &request.source_item_instance_id,
        tool,
        flask,
    )
    .await
}

/// Lock `source`, its ancestry to the holder's backpack, an optional `tool` with its ancestry
/// and an optional `extra` item, then recheck the paths, the owner and the backpack slot.
pub(super) async fn lock_carried_path(
    connection: &mut PgConnection,
    holder: &[u8; 16],
    backpack: &[u8; 16],
    world: &[u8; 16],
    source: &[u8; 16],
    tool: Option<[u8; 16]>,
    extra: Option<[u8; 16]>,
) -> Result<()> {
    let path = use_path(connection, source).await?;
    if !(2..=9).contains(&path.len()) || path.last() != Some(backpack) {
        return Err(ItemUseError::StaleState);
    }
    let mut items = path.clone();
    let tool_path = if let Some(tool) = tool {
        let tool_path = use_path(connection, &tool).await?;
        if !(2..=9).contains(&tool_path.len()) || tool_path.last() != Some(backpack) {
            return Err(ItemUseError::StaleState);
        }
        items.extend_from_slice(&tool_path);
        Some((tool, tool_path))
    } else {
        None
    };
    items.extend(extra);
    items.sort_unstable();
    items.dedup();
    // A JSON array avoids database UUID driver conversions; these are server-owned IDs.
    let ids: Vec<_> = items
        .iter()
        .map(|id| id.iter().map(|b| format!("{b:02x}")).collect::<String>())
        .collect();
    let locked: Vec<String> = sqlx::query_scalar(
        "SELECT i.item_instance_id::text FROM game_item_instances i \
          WHERE i.item_instance_id IN (SELECT value::uuid FROM jsonb_array_elements_text($1::jsonb)) \
            AND i.lifecycle = 1 AND i.world_id = encode($2,'hex')::uuid \
          ORDER BY i.item_instance_id FOR UPDATE OF i",
    )
    .bind(sqlx::types::Json(ids))
    .bind(world.as_slice())
    .fetch_all(&mut *connection)
    .await?;
    if locked.len() != items.len()
        || use_path(connection, source).await? != path
    {
        return Err(ItemUseError::StaleState);
    }
    if let Some((tool, path)) = tool_path
        && use_path(connection, &tool).await? != path
    {
        return Err(ItemUseError::StaleState);
    }
    let owner: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_item_loop_owner(encode($1,'hex')::uuid) \
          WHERE owner_character = encode($2,'hex')::uuid AND depth BETWEEN 1 AND 8)",
    )
    .bind(source.as_slice())
    .bind(holder.as_slice())
    .fetch_one(&mut *connection)
    .await?;
    if !owner {
        return Err(ItemUseError::StaleState);
    }
    // Container-slot custody is checked again after all item locks (§4.2). The runtime
    // role holds no UPDATE on game_item_container_slots, so the row lock is taken on
    // the backpack's item row, as every runtime backpack writer does.
    let slot = sqlx::query(
        "SELECT 1 FROM game_item_container_slots s \
           JOIN game_item_instances i ON i.item_instance_id = s.item_instance_id \
          WHERE s.character_id = encode($1,'hex')::uuid \
            AND s.item_instance_id = encode($2,'hex')::uuid FOR UPDATE OF i",
    )
    .bind(holder.as_slice())
    .bind(backpack.as_slice())
    .fetch_optional(connection)
    .await?;
    if slot.is_none() {
        return Err(ItemUseError::StaleState);
    }
    Ok(())
}
