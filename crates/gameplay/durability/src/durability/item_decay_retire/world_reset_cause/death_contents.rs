//! Every retained death-tree edge shares the Ground root's atomic retirement.
use super::*;

pub(in crate::durability::item_decay_retire) async fn retire_death_contents(
    tx: &mut mint_audit::Type2Transaction<'_>,
    permit: &SpellLanePermit,
    fence: &CorpseDecayFence,
    node: &NodeIncarnationProof,
    step: DecayRetireStep,
    parent: &Admitted,
) -> std::result::Result<(), DurabilityError> {
    if !matches!(
        step.cause,
        RetireCause::WorldReset(_) | RetireCause::ActivityRetirement(_)
    ) || !step.is_corpse_step()
        || !crate::durability::world_reset::ground_step::is_death_container(
            tx,
            step.corpse_item_instance_id,
        )
        .await?
    {
        return Ok(());
    }
    let tree =
        crate::durability::world_reset::ground_step::death_tree(tx, step.item_instance_id).await?;
    let ids: Vec<_> = tree
        .iter()
        .map(|entry| sqlx::types::Uuid::from_bytes(entry.item))
        .collect();
    // The root is already locked by admission. Lock every descendant before
    // re-reading the edges, then mutate only the exact revalidated graph.
    let rows = sqlx::query("SELECT i.item_instance_id::text,i.world_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.lifecycle,(SELECT coalesce(max(key_number),0)::bigint FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) AS key_number,game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id FROM game_item_instances i WHERE i.item_instance_id=ANY($1) ORDER BY i.item_instance_id FOR UPDATE OF i")
        .bind(ids)
        .fetch_all(&mut ***tx).await?;
    if rows.len() != tree.len()
        || crate::durability::world_reset::ground_step::death_tree(tx, step.item_instance_id)
            .await?
            != tree
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut locked = std::collections::BTreeMap::new();
    for row in rows {
        locked.insert(uuid_text(row.try_get("item_instance_id")?)?, row);
    }
    for entry in tree {
        let row = locked
            .remove(&entry.item)
            .ok_or(DurabilityError::InvalidStoredState)?;
        if row.try_get::<i16, _>("lifecycle")? != 1
            || uuid_text(row.try_get("world_id")?)? != parent.world_id
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let child = DecayRetireStep {
            item_instance_id: entry.item,
            ..step
        };
        let admitted = Admitted {
            world_id: parent.world_id,
            runtime_scope: parent.runtime_scope,
            definition: OneItemTypedDefinitionRevisionV1 {
                family: row.try_get("definition_family")?,
                production_key: row.try_get("definition_production_key")?,
                revision_ref: row.try_get("definition_revision_ref")?,
            },
            quantity_before: u32::try_from(row.try_get::<i64, _>("quantity")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            key_number: u32::try_from(row.try_get::<i64, _>("key_number")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            placement_ordinal: Some(entry.ordinal),
            death_entry: Some(entry),
            corpse_ground: parent.corpse_ground.clone(),
            deadline_unix_ms: 0,
            database_now_unix_ms: parent.database_now_unix_ms,
        };
        let transaction_id = uuid_text(row.try_get("transaction_id")?)?;
        let event_id = uuid_text(row.try_get("event_id")?)?;
        let tuple = tx.tuple();
        let envelope = audit::encode_decay_retire_event(
            DecayRetireEventIdentity {
                event_id,
                transaction_id,
                occurred_at_unix_ms: admitted.database_now_unix_ms,
                server_build_id: SERVER_BUILD_ID,
            },
            decay_message(child, fence, &admitted),
            tuple.tuple(),
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        let fact = node.fact();
        let reservation = Reservation {
            cause: step.cause,
            corpse_item_instance_id: step.corpse_item_instance_id,
            fence_generation: fence.scope_ownership_generation.get(),
            transaction_id,
            event_id,
            occurred_at_unix_ms: admitted.database_now_unix_ms,
            deadline_unix_ms: 0,
            quantity_before: admitted.quantity_before,
            placement_ordinal: admitted.placement_ordinal,
            envelope,
            fence_node_id: *fact.node_id().as_bytes(),
            fence_registration_revision: fact.registration_revision(),
            work_units_used: 0,
        };
        retirement_workflow::insert_reservation(tx, child, &admitted, &reservation, tuple).await?;
        apply_death_entry_retire(tx, permit, child, &admitted, &reservation, tuple).await?;
    }
    Ok(())
}

async fn apply_death_entry_retire(
    tx: &mut mint_audit::Type2Transaction<'_>,
    permit: &SpellLanePermit,
    step: DecayRetireStep,
    admitted: &Admitted,
    reservation: &Reservation,
    tuple: SelectedType2Tuple,
) -> std::result::Result<(), DurabilityError> {
    use crate::durability::world_reset::ground_step::DeathEntryKind;
    // Keep the same SpellLane check immediately before the ordered custody,
    // Item, receipt and audit writes; a refusal aborts the caller's transaction.
    permit.check_scope(admitted.runtime_scope)?;
    let entry = admitted
        .death_entry
        .ok_or(DurabilityError::InvalidStoredState)?;
    let retired: bool = sqlx::query_scalar(
        "SELECT game_player_death_retire_entry(encode($1,'hex')::uuid, $2::text::numeric(20,0), \
         encode($3,'hex')::uuid, encode($4,'hex')::uuid, $5, $6, $7, $8, $9)",
    )
    .bind(step.item_instance_id.as_slice())
    .bind(reservation.fence_generation.to_string())
    .bind(reservation.transaction_id.as_slice())
    .bind(entry.parent.as_slice())
    .bind(match entry.kind {
        DeathEntryKind::Direct => 1_i16,
        DeathEntryKind::Nested => 2_i16,
    })
    .bind(EVENT_TYPE_ID)
    .bind(i64::from(tuple.schema_revision()))
    .bind(tuple.retention_profile_id())
    .bind(mint_audit::AUDIT_RETENTION_P90D_MS)
    .fetch_one(&mut ***tx)
    .await?;
    if !retired {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
