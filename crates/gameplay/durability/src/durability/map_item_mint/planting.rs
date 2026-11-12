//! A picked-up planted pot retains the map owner's original absolute deadlines.
use super::*;

pub(super) async fn inherit_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenMint,
    reservation: &Reservation,
) -> std::result::Result<(), DurabilityError> {
    let request = &frozen.request;
    let Some(id) = request.item.definition.production_key
        .strip_prefix("oteryn:item.tibia.i").and_then(|id| id.parse::<u32>().ok()) else { return Ok(()); };
    if !matches!(id, 324 | 330 | 14031 | 14032) { return Ok(()); }
    let mut placement = request.base_bundle_digest.to_vec();
    placement.extend_from_slice(&request.placement_key.to_be_bytes());
    let row = sqlx::query("SELECT state_item_id,decay_states,decay_delays,decay_step,decay_at_unix_ms,content_generation FROM game_terrain_tool_transforms WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND placement=$3 AND reset_epoch=$4::text::bigint FOR UPDATE")
        .bind(reservation.world_id.as_slice()).bind(reservation.channel_id.as_slice())
        .bind(&placement).bind(request.reset_epoch.to_string()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(()); };
    let state: i64 = row.try_get("state_item_id")?;
    let states: Vec<i64> = row.try_get("decay_states")?;
    let delays: Vec<i64> = row.try_get("decay_delays")?;
    let step = usize::try_from(row.try_get::<i32, _>("decay_step")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let Some(mut deadline) = row.try_get::<Option<i64>, _>("decay_at_unix_ms")? else { return Ok(()); };
    let expected = oteryn_gameplay_items::event_planting::decay_steps(id);
    if state != i64::from(id) || request.quantity != 1 || expected.is_empty()
        || states.len() != delays.len() || step.checked_add(expected.len()) != Some(states.len())
        || expected.iter().enumerate().any(|(index, rule)|
            states[step + index] != i64::from(rule.target) || delays[step + index] != rule.after_ms) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let digest: Vec<u8> = row.try_get("content_generation")?;
    let mut before = id;
    for (index, rule) in expected.iter().enumerate() {
        if index > 0 {
            deadline = deadline.checked_add(rule.after_ms).ok_or(DurabilityError::InvalidStoredState)?;
        }
        // The closed source plant states are movable, nonblocking pots. The pinned event
        // profile capture binds these same physical facts and revision before map admission.
        sqlx::query("INSERT INTO game_spell_field_temporal_schedules(item_instance_id,source_stage,source_transaction_id,event_source_transaction_id,map_source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,target_family,target_production_key,target_revision,content_digest,blocks_movement,blocks_projectile,immovable_block_solid) VALUES(encode($1,'hex')::uuid,$2,NULL,NULL,encode($3,'hex')::uuid,1,$4,$5,'Item',$6,$7,'Item',$8,$7,$9,false,false,false)")
            .bind(frozen.item_instance_id.as_slice()).bind(i32::try_from(index + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(frozen.transaction_id.as_slice()).bind(rule.after_ms).bind(deadline)
            .bind(format!("oteryn:item.tibia.i{before}")).bind(&request.item.definition.revision_ref)
            .bind(format!("oteryn:item.tibia.i{}", rule.target)).bind(&digest).execute(&mut **tx).await?;
        before = rule.target;
    }
    Ok(())
}
