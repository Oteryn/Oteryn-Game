//! Ground-only before-state, checked again in every freeze/commit pass.
use super::*;

pub(in crate::durability::item_decay_retire) async fn admit(
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CorpseDecayFence,
    step: DecayRetireStep,
    reset: RetiringWorldReset,
) -> Pass<Admitted> {
    let channels =
        match crate::durability::world_reset::census::drained_channels(root, tx, node, reset)
            .await?
        {
            crate::durability::world_reset::census::WorldResetDrain::Drained(channels) => channels,
            crate::durability::world_reset::census::WorldResetDrain::NotCurrent => {
                return Ok(Err(DecayRetireError::AuthorityRejected));
            }
            crate::durability::world_reset::census::WorldResetDrain::CapacityExceeded => {
                return Ok(Err(DecayRetireError::CapacityExceeded));
            }
        };
    if !channels.contains(fence) {
        return Ok(Err(DecayRetireError::AuthorityRejected));
    }
    let item = sqlx::query("SELECT definition_family,definition_production_key,definition_revision_ref,quantity,(SELECT coalesce(max(key_number),0)::bigint FROM game_item_keys k WHERE k.item_instance_id=game_item_instances.item_instance_id) AS key_number FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND lifecycle=1 FOR UPDATE")
        .bind(step.item_instance_id.as_slice()).bind(reset.world.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    let Some(item) = item else {
        return Ok(Err(DecayRetireError::Refused(
            DecayRetireRefusal::NotInCorpse,
        )));
    };
    let ground = sqlx::query("SELECT g.spatial_position,g.corpse_ref,g.map_revision,g.content_revision,g.native_room_placement_context,g.runtime_scope_ownership_generation::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS database_now FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.item_instance_id=encode($1,'hex')::uuid AND g.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF i")
        .bind(step.corpse_item_instance_id.as_slice()).bind(reset.world.as_bytes().as_slice())
        .bind(fence.channel_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(ground) = ground else {
        return Ok(Err(DecayRetireError::Refused(
            DecayRetireRefusal::NotACorpse,
        )));
    };
    // A nested or character-container child is not an admitted Ground shape.
    // Refuse before attempting any child or the root, including nested children
    // of a direct corpse entry. These bounded families have indexed parent keys.
    crate::durability::world_reset::ground_step::validate_root(tx, step.corpse_item_instance_id)
        .await?;
    let entries = sqlx::query("SELECT item_instance_id::text,placement_ordinal::text FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY placement_ordinal LIMIT 17")
        .bind(step.corpse_item_instance_id.as_slice()).fetch_all(&mut **tx).await?;
    if entries.len() > CORPSE_CONTAINER_ENTRIES_MAX {
        return Err(DurabilityError::InvalidStoredState);
    }
    let placement_ordinal = if step.is_corpse_step() {
        if !entries.is_empty() {
            return Ok(Err(DecayRetireError::Refused(
                DecayRetireRefusal::EntriesRemain,
            )));
        }
        None
    } else {
        let Some(first) = entries.first() else {
            return Ok(Err(DecayRetireError::Refused(
                DecayRetireRefusal::NotInCorpse,
            )));
        };
        if uuid_text(first.try_get("item_instance_id")?)? != step.item_instance_id {
            return Ok(Err(DecayRetireError::Refused(
                DecayRetireRefusal::EntriesRemain,
            )));
        }
        Some(
            first
                .try_get::<String, _>("placement_ordinal")?
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
    };
    Ok(Ok(Admitted {
        world_id: *reset.world.as_bytes(),
        channel_id: *fence.channel_id.as_bytes(),
        definition: OneItemTypedDefinitionRevisionV1 {
            family: item.try_get("definition_family")?,
            production_key: item.try_get("definition_production_key")?,
            revision_ref: item.try_get("definition_revision_ref")?,
        },
        quantity_before: u32::try_from(item.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        key_number: u32::try_from(item.try_get::<i64, _>("key_number")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        placement_ordinal,
        corpse_ground: OneItemGroundV1 {
            world_id: reset.world.as_bytes().to_vec(),
            channel_id: fence.channel_id.as_bytes().to_vec(),
            spatial_position: ground.try_get("spatial_position")?,
            corpse_ref: ground.try_get("corpse_ref")?,
            map_revision: ground.try_get("map_revision")?,
            content_revision: ground.try_get("content_revision")?,
            native_room_placement_context: ground.try_get("native_room_placement_context")?,
            runtime_scope_ownership_generation: ground
                .try_get::<String, _>("runtime_scope_ownership_generation")?
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        },
        deadline_unix_ms: 0,
        database_now_unix_ms: ground.try_get("database_now")?,
    }))
}
