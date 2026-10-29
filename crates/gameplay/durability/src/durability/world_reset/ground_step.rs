//! The next Ground step is derived from durable locations, never a caller's list.
use super::*;
use crate::durability::item_decay_retire::DecayRetireStep;
use crate::durability::item_mint::uuid_text;

pub(super) async fn next(
    tx: &mut Transaction<'_, Postgres>,
    reset: RetiringWorldReset,
) -> Result<Option<([u8; 16], DecayRetireStep)>> {
    // House custody survives a map reset.  The house tile catalogue is the durable
    // authority here; relying on the retiring map's in-memory facts would make a
    // restart capable of deleting property before the replacement map is loaded.
    let row = sqlx::query("SELECT g.item_instance_id::text,g.channel_id::text FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.world_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND NOT EXISTS(SELECT 1 FROM game_house_tiles h WHERE h.world_id=g.world_id AND h.spatial_position=g.spatial_position) ORDER BY g.channel_id,g.item_instance_id LIMIT 1")
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
    let death_container = is_death_container(tx, root).await?;
    let invalid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid OR parent_item_instance_id IN(SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)) OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid OR parent_item_instance_id IN(SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)) OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id IN(SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid))")
        .bind(root.as_slice()).fetch_one(&mut **tx).await?;
    if invalid && !death_container {
        return Err(DurabilityError::InvalidStoredState);
    }
    if death_container {
        // Death drops admit direct contents only. Keep all other shape guards,
        // including children of direct entries, as strict as ordinary Ground.
        let invalid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid OR parent_item_instance_id IN(SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)) OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id IN(SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)) OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid OR parent_item_instance_id IN(SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid))")
            .bind(root.as_slice()).fetch_one(&mut **tx).await?;
        if invalid {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}

pub(in crate::durability) async fn is_death_container(
    tx: &mut Transaction<'_, Postgres>,
    root: [u8; 16],
) -> Result<bool> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_player_death_item_records r JOIN game_item_ground_locations g USING(item_instance_id) WHERE r.item_instance_id=encode($1,'hex')::uuid AND r.operation=1 AND r.source_slot=9 AND g.world_id=r.world_id AND g.channel_id=r.channel_id AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(r.transaction_id))")
        .bind(root.as_slice()).fetch_one(&mut **tx).await?)
}
