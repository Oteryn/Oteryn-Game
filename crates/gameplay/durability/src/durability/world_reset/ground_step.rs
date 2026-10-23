//! The next Ground step is derived from durable locations, never a caller's list.
use super::*;
use crate::durability::item_decay_retire::DecayRetireStep;
use crate::durability::item_mint::uuid_text;

pub(super) async fn next(
    tx: &mut Transaction<'_, Postgres>,
    reset: RetiringWorldReset,
) -> Result<Option<([u8; 16], DecayRetireStep)>> {
    let row = sqlx::query("SELECT g.item_instance_id::text,g.channel_id::text FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.world_id=encode($1,'hex')::uuid AND i.lifecycle=1 ORDER BY g.channel_id,g.item_instance_id LIMIT 1")
        .bind(reset.world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let root = uuid_text(row.try_get("item_instance_id")?)?;
    let channel = uuid_text(row.try_get("channel_id")?)?;
    validate_root(tx, root).await?;
    let item: Option<String> = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY placement_ordinal LIMIT 1")
        .bind(root.as_slice()).fetch_optional(&mut **tx).await?;
    let item = item
        .map(|item| uuid_text(&item))
        .transpose()?
        .unwrap_or(root);
    Ok(Some((
        channel,
        DecayRetireStep::for_world_reset(reset, root, item),
    )))
}

pub(in crate::durability) async fn validate_root(
    tx: &mut Transaction<'_, Postgres>,
    root: [u8; 16],
) -> Result<()> {
    let invalid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid OR parent_item_instance_id IN(SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)) OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid OR parent_item_instance_id IN(SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)) OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id IN(SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid))")
        .bind(root.as_slice()).fetch_one(&mut **tx).await?;
    if invalid {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
