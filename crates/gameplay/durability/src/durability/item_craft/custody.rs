//! Locked input custody, shared by atomic crafts and their SQL completion proofs.
use super::*;

pub(super) async fn source(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    item: [u8; 16],
) -> Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let cell = request
        .ground_inputs
        .iter()
        .find(|(id, _)| *id == item)
        .map(|(_, cell)| cell);
    Ok(sqlx::query("SELECT to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,c.location AS custody,uuid_send(coalesce((c.location->'row'->>'parent_item_instance_id')::uuid,i.item_instance_id)) AS parent,coalesce(c.location->'row'->>'placement_ordinal','1') AS ordinal FROM game_item_instances i CROSS JOIN LATERAL (SELECT game_item_craft_custody(i.item_instance_id) AS location) c WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND ($6 OR c.location->>'kind'='main') AND ((c.location->>'kind' IN ('main','nested') AND EXISTS(SELECT 1 FROM game_item_loop_owner(i.item_instance_id) WHERE owner_character=encode($2,'hex')::uuid AND depth BETWEEN 1 AND 8)) OR (c.location->>'kind'='equipment' AND c.location->'row'->>'character_id'=encode($2,'hex')::uuid::text) OR (c.location->>'kind'='ground' AND c.location->'row'->>'channel_id'=encode($4,'hex')::uuid::text AND c.location->'row'->>'world_id'=encode($3,'hex')::uuid::text AND decode(substr(c.location->'row'->>'spatial_position',3),'hex')=$5))")
        .bind(item.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(cell).bind(request.target.is_some()).fetch_optional(&mut ***tx).await?)
}

pub(super) async fn remove(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    item: [u8; 16],
    row: &sqlx::postgres::PgRow,
) -> Result<(), DurabilityError> {
    let location: Value = row.try_get("custody")?;
    let sql = match location["kind"].as_str() {
        Some("main") => {
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid"
        }
        Some("nested") => {
            "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid"
        }
        Some("equipment") => {
            "DELETE FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid"
        }
        Some("ground") => {
            "DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid"
        }
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    sqlx::query(sql)
        .bind(item.as_slice())
        .execute(&mut ***tx)
        .await?;
    Ok(())
}
