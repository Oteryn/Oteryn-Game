//! Death-dropped direct contents share the Ground root's atomic reset commit.
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
    let rows = sqlx::query("SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.lifecycle,e.placement_ordinal::text,(SELECT coalesce(max(key_number),0)::bigint FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) AS key_number FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY i.item_instance_id LIMIT $2 FOR UPDATE OF i,e")
        .bind(step.item_instance_id.as_slice())
        .bind(i64::from(crate::durability::item_transfer::GAMEITEM01_CONTAINER_ENTRIES_MAX) + 1)
        .fetch_all(&mut ***tx).await?;
    if rows.len() > crate::durability::item_transfer::GAMEITEM01_CONTAINER_ENTRIES_MAX as usize {
        return Err(DurabilityError::InvalidStoredState);
    }
    for row in rows {
        if row.try_get::<i16, _>("lifecycle")? != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let child = DecayRetireStep {
            item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
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
            placement_ordinal: Some(
                row.try_get::<String, _>("placement_ordinal")?
                    .parse()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            ),
            corpse_ground: parent.corpse_ground.clone(),
            deadline_unix_ms: 0,
            database_now_unix_ms: parent.database_now_unix_ms,
        };
        let identity = sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id")
            .fetch_one(&mut ***tx).await?;
        let transaction_id = uuid_text(identity.try_get("transaction_id")?)?;
        let event_id = uuid_text(identity.try_get("event_id")?)?;
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
        apply_retire(tx, permit, child, &admitted, &reservation, tuple).await?;
    }
    Ok(())
}
