//! The native USE owner consumes an immovable Store lamp directly from its real
//! Inbox location. The Hireling owner has already proven its named House fence;
//! its placement and this burn commit in the same physical transaction.
use super::*;

pub(super) async fn write_if_inbox(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
    tuple: SelectedType2Tuple,
) -> Result<Option<ItemUseOutcome>> {
    let ItemUseKind::HirelingLamp { origin_channel } = request.kind else {
        return Err(ItemUseError::InvalidInput);
    };
    if request.source_quantity_before != 1 || request.flask != ItemUseFlaskPlan::None
        || request.source_definition.family != "Item"
        || request.source_definition.production_key != crate::durability::house_hireling::HIRELING_LAMP_KEY
    { return Err(ItemUseError::InvalidInput); }
    let world = fence.runtime_scope.world_id();
    let row = sqlx::query(
        "SELECT l.ordinal::text AS ordinal, i.quantity, i.lifecycle, i.definition_family, \
                i.definition_production_key, i.definition_revision_ref \
         FROM game_item_character_inbox_locations l JOIN game_item_instances i USING(item_instance_id) \
         WHERE l.item_instance_id=encode($1,'hex')::uuid AND l.character_id=encode($2,'hex')::uuid \
           AND l.world_id=encode($3,'hex')::uuid AND i.world_id=l.world_id FOR UPDATE OF i",
    ).bind(request.source_item_instance_id.as_slice()).bind(fence.character_id.as_bytes().as_slice())
     .bind(world.as_bytes().as_slice()).fetch_optional(&mut *connection).await?;
    let Some(row) = row else { return Ok(None); };
    if row.try_get::<i64,_>("quantity")? != 1 || row.try_get::<i16,_>("lifecycle")? != 1
        || row.try_get::<String,_>("definition_family")? != request.source_definition.family
        || row.try_get::<String,_>("definition_production_key")? != request.source_definition.production_key
        || row.try_get::<String,_>("definition_revision_ref")? != request.source_definition.revision_ref
    { return Err(ItemUseError::StaleState); }
    let ordinal = numeric_u64(&row, "ordinal")?;
    let house = house_scope(connection, fence).await?;
    let state = |quantity, lifecycle| OneItemStateV1 {
        item_instance_id: request.source_item_instance_id.to_vec(), world_id: world.as_bytes().to_vec(),
        definition: Some(definition_message(&request.source_definition)), quantity, lifecycle, key_number: 0,
    };
    let envelope = encode_item_use_event(ItemUseEventIdentity {
        event_id: request.event_id, transaction_id: request.transaction_id,
        occurred_at_unix_ms: request.occurred_at_unix_ms, server_build_id: &request.server_build_id,
    }, OneItemUseV1 {
        source_before: Some(state(1, ITEM_LIFECYCLE_LIVE)),
        source_after: Some(state(0, ITEM_LIFECYCLE_RETIRED)),
        source_entry: None, source_inbox_ordinal: Some(ordinal), house_key: Some(house.clone()),
        cause: Some(OneItemUseCauseV1 {
            kind: request.kind.code(), content_revision: request.content_revision.clone(),
            ruleset_revision: request.ruleset_revision.clone(), sim_revision: request.sim_revision.clone(),
            command_ref: Some(OneItemCommandRefV1 { game_session_id: fence.game_session_id.as_bytes().to_vec(), command_id: request.command_id }),
        }),
        character_id: fence.character_id.as_bytes().to_vec(), world_id: world.as_bytes().to_vec(),
        channel_id: origin_channel.to_vec(), runtime_scope_ownership_generation: fence.scope_ownership_generation.get(),
        expected_session_generation: fence.connection_generation.get(), expected_game_session_id: fence.game_session_id.as_bytes().to_vec(),
        flask: None, flask_missing: false,
    }, tuple.tuple()).map_err(|_| ItemUseError::InvalidInput)?;
    sqlx::query(
        "INSERT INTO game_item_use_records(transaction_id,event_id,cause_kind,game_session_id,command_id, \
         content_revision,ruleset_revision,sim_revision,request_binding,character_id,world_id,channel_id, \
         runtime_scope_ownership_generation,expected_session_generation,shape,source_item_instance_id, \
         source_definition_family,source_definition_production_key,source_definition_revision_ref,source_kind, \
         source_parent_item_instance_id,source_placement_ordinal,source_quantity_before,source_quantity_after, \
         occurred_at,envelope_sha256,source_house_key) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,470,encode($3,'hex')::uuid,$4::text::numeric, \
          $5,$6,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,encode($11,'hex')::uuid, \
          $12::text::numeric,$13::text::numeric,1,encode($14,'hex')::uuid,$15,$16,$17,2,NULL, \
          $18::text::numeric,1,0,$19,sha256($20),$21)",
    ).bind(request.transaction_id.as_slice()).bind(request.event_id.as_slice())
     .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command_id.to_string())
     .bind(&request.content_revision).bind(&request.ruleset_revision).bind(&request.sim_revision)
     .bind(request.binding().as_slice()).bind(fence.character_id.as_bytes().as_slice())
     .bind(world.as_bytes().as_slice()).bind(origin_channel.as_slice())
     .bind(fence.scope_ownership_generation.get().to_string()).bind(fence.connection_generation.get().to_string())
     .bind(request.source_item_instance_id.as_slice()).bind(&request.source_definition.family)
     .bind(&request.source_definition.production_key).bind(&request.source_definition.revision_ref)
     .bind(ordinal.to_string()).bind(request.occurred_at_unix_ms).bind(envelope.as_slice()).bind(&house)
     .execute(&mut *connection).await?;
    let changed = sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid AND quantity=1 AND lifecycle=1")
        .bind(request.source_item_instance_id.as_slice()).bind(request.transaction_id.as_slice())
        .execute(&mut *connection).await?.rows_affected();
    if changed != 1 { return Err(ItemUseError::StaleState); }
    sqlx::query("SELECT game_hireling_inbox_lamp_unlink(encode($1,'hex')::uuid)")
        .bind(request.transaction_id.as_slice()).execute(&mut *connection).await?;
    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count, \
         event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,$3,$4,$5,encode($6,'hex')::uuid,$7,$7+$8,$9,sha256($9),1)",
    ).bind(request.event_id.as_slice()).bind(request.transaction_id.as_slice())
     .bind(i64::from(mint_audit::EVENT_TYPE_ID)).bind(i64::from(tuple.schema_revision()))
     .bind(tuple.retention_profile_id()).bind(request.source_item_instance_id.as_slice())
     .bind(request.occurred_at_unix_ms).bind(mint_audit::AUDIT_RETENTION_P90D_MS).bind(envelope.as_slice())
     .execute(&mut *connection).await?;
    Ok(Some(ItemUseOutcome::Written { shape: ItemUseShape::Burn }))
}

/// Read the exact named scope proved by the caller; an origin Channel never
/// supplies House identity to the item-use audit.
pub(super) async fn house_scope(connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence) -> Result<String> {
    let RuntimeScopeRefV1::Instance { world_id, instance_id } = fence.runtime_scope else {
        return Err(ItemUseError::AuthorityRejected);
    };
    let house: Option<String> = sqlx::query_scalar(
        "SELECT runtime_scope_house_key FROM game_durability_reconnect_sessions \
         WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid \
           AND world_id=encode($3,'hex')::uuid AND runtime_scope_kind=2 \
           AND runtime_scope_world_id=world_id AND runtime_scope_channel_id IS NULL \
           AND runtime_scope_instance_id=encode($4,'hex')::uuid AND runtime_scope_house_key IS NOT NULL \
           AND current_generation=$5::text::numeric AND character_lease_generation=$6::text::numeric \
           AND scope_ownership_generation=$7::text::numeric AND session_state IN(1,2) FOR SHARE",
    ).bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
     .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice())
     .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
     .bind(fence.scope_ownership_generation.get().to_string()).fetch_optional(connection).await?;
    let house = house.ok_or(ItemUseError::AuthorityRejected)?;
    if crate::durability::house_scope_handoff::HouseId::new(world_id, &house)
        .is_none_or(|named| named.instance_id() != instance_id) {
        return Err(ItemUseError::AuthorityRejected);
    }
    Ok(house)
}
