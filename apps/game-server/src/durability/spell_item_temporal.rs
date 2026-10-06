//! Absolute ItemType decay on the existing durable Item owner. Schedules are
//! minted in the original cast transaction, not inferred from current names.
use super::spell_item_transaction::{
    SpellItemAuthority, SpellItemError, SpellItemScopeAuthority, check_scope_transaction,
    check_transaction,
};
use super::spell_items_abi::{QualifiedItemDefinition, SpellItemTransactionRequest};
use sqlx::{Postgres, Row, Transaction};

pub(crate) async fn schedule_minted_item_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
    definition: &QualifiedItemDefinition,
    item: [u8; 16],
    ordinal: u32,
    occurred_at: i64,
) -> Result<(), SpellItemError> {
    let Some(decay) = &definition.decay else {
        return Ok(());
    };
    check_transaction(tx, authority).await?;
    if decay.duration_millis == 0
        || definition.stack_maximum != 1
        || definition.container_capacity.is_some()
        || definition.content_generation_digest != authority.compatible_content_digest()
    {
        return Err(SpellItemError::Rejected(
            "unsupported ItemType decay source",
        ));
    }
    let expires = occurred_at
        .checked_add(i64::from(decay.duration_millis))
        .ok_or(SpellItemError::Rejected("ItemType deadline overflow"))?;
    sqlx::query("INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,target_family,target_production_key,target_revision,content_digest) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
        .bind(item.as_slice()).bind(request.transaction_id.as_slice()).bind(i32::try_from(ordinal).map_err(|_|SpellItemError::Rejected("ItemType source ordinal"))?)
        .bind(i64::from(decay.duration_millis)).bind(expires).bind(&definition.definition.family)
        .bind(&definition.definition.production_key).bind(&definition.definition.revision_ref)
        .bind(decay.target.as_ref().map(|t|t.family.as_str())).bind(decay.target.as_ref().map(|t|t.production_key.as_str()))
        .bind(decay.target.as_ref().map(|t|t.revision_ref.as_str())).bind(definition.content_generation_digest.as_slice())
        .execute(&mut **tx).await?;
    Ok(())
}

/// Executes original source deadlines on the one actual Item store. Inventory
/// is eligible only under a currently active Character in this owned Channel;
/// an offline or foreign-scope item keeps its original overdue schedule.
pub(crate) async fn drain_due_items_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    limit: usize,
) -> Result<usize, SpellItemError> {
    if limit > 500 {
        return Err(SpellItemError::Rejected("Item timer batch bound"));
    }
    if limit == 0 {
        return Ok(0);
    }
    check_scope_transaction(tx, authority).await?;
    let world = authority.world_bytes();
    let channel = authority.channel_bytes();
    let rows=sqlx::query(r#"SELECT i.item_instance_id::text,i.state_revision::text,i.quantity,
       p.target_family,p.target_production_key,p.target_revision,g.stack_ordinal,
       e.character_id::text AS bag_character,e.parent_item_instance_id::text AS parent,e.placement_ordinal::text AS ordinal,
       eq.character_id::text AS equipped_character,eq.slot,es.revision::text AS equipment_revision,
       sess.game_session_id::text AS session,sess.character_lease_generation::text AS lease,sess.current_generation::text AS connection,
       game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id
      FROM game_spell_item_temporal_schedules p JOIN game_item_instances i USING(item_instance_id)
      LEFT JOIN game_item_ground_locations g USING(item_instance_id)
      LEFT JOIN game_item_container_entries e USING(item_instance_id)
      LEFT JOIN game_item_container_slots bag ON bag.item_instance_id=e.parent_item_instance_id AND bag.character_id=e.character_id
      LEFT JOIN game_character_equipment_slots eq USING(item_instance_id)
      LEFT JOIN game_character_equipment_state es ON es.character_id=eq.character_id
      LEFT JOIN game_durability_admission_character_guards cg ON cg.character_id=COALESCE(e.character_id,eq.character_id)
      LEFT JOIN game_durability_reconnect_sessions sess ON sess.game_session_id=cg.holder_game_session_id
      WHERE i.lifecycle=1 AND i.quantity=1 AND i.world_id=encode($1,'hex')::uuid
       AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=(p.definition_family,p.definition_production_key,p.definition_revision)
       AND p.expires_at_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
       AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts done WHERE done.item_instance_id=i.item_instance_id)
       AND ((g.world_id=i.world_id AND g.channel_id=encode($2,'hex')::uuid AND g.runtime_scope_ownership_generation=$3::text::numeric)
         OR (((e.world_id=i.world_id AND bag.character_id=e.character_id) OR eq.world_id=i.world_id)
          AND cg.eligible AND cg.lease_generation=sess.character_lease_generation AND sess.session_state=2
          AND sess.character_id=cg.character_id AND sess.runtime_scope_kind=1 AND sess.runtime_scope_world_id=i.world_id
          AND sess.runtime_scope_channel_id=encode($2,'hex')::uuid AND sess.scope_ownership_generation=$3::text::numeric))
      ORDER BY p.expires_at_unix_ms,i.item_instance_id LIMIT $4 FOR UPDATE OF i,p"#)
        .bind(world.as_slice()).bind(channel.as_slice()).bind(authority.generation().to_string()).bind(i64::try_from(limit).map_err(|_|SpellItemError::Rejected("Item timer batch bound"))?).fetch_all(&mut **tx).await?;
    let mut changed = 0;
    for row in rows {
        let item: String = row.try_get("item_instance_id")?;
        let transaction: String = row.try_get("transaction_id")?;
        let event: String = row.try_get("event_id")?;
        let revision: String = row.try_get("state_revision")?;
        let parent: Option<String> = row.try_get("parent")?;
        let slot: Option<i16> = row.try_get("slot")?;
        let stack: Option<i64> = row.try_get("stack_ordinal")?;
        let kind: i16 = if stack.is_some() {
            1
        } else if parent.is_some() {
            2
        } else if slot.is_some() {
            3
        } else {
            return Err(SpellItemError::Rejected(
                "Item timer has no supported exact custody",
            ));
        };
        let character: Option<String> = if kind == 2 {
            row.try_get("bag_character")?
        } else if kind == 3 {
            row.try_get("equipped_character")?
        } else {
            None
        };
        let session: Option<String> = if kind == 1 {
            None
        } else {
            row.try_get("session")?
        };
        let lease: Option<String> = if kind == 1 {
            None
        } else {
            row.try_get("lease")?
        };
        let connection: Option<String> = if kind == 1 {
            None
        } else {
            row.try_get("connection")?
        };
        let ordinal: Option<String> = row.try_get("ordinal")?;
        let equipment_revision: Option<String> = row.try_get("equipment_revision")?;
        sqlx::query("INSERT INTO game_spell_item_temporal_receipts(transaction_id,event_id,item_instance_id,world_id,channel_id,ownership_generation,state_revision_before,quantity_before,occurred_at_unix_ms,custody_kind,character_id,game_session_id,character_lease_generation,connection_generation,source_parent,source_ordinal,source_slot,equipment_revision_before,source_stack_ordinal) VALUES($1::uuid,$2::uuid,$3::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7::text::numeric,1,floor(extract(epoch FROM statement_timestamp())*1000)::bigint,$8,$9::uuid,$10::uuid,$11::text::numeric,$12::text::numeric,$13::uuid,$14::text::numeric,$15,$16::text::numeric,$17)")
            .bind(&transaction).bind(&event).bind(&item).bind(world.as_slice()).bind(channel.as_slice()).bind(authority.generation().to_string())
            .bind(&revision).bind(kind).bind(&character).bind(&session).bind(&lease).bind(&connection).bind(&parent).bind(&ordinal).bind(slot).bind(&equipment_revision).bind(stack)
            .execute(&mut **tx).await?;
        let target: Option<String> = row.try_get("target_production_key")?;
        let affected = if let Some(target) = target {
            sqlx::query("UPDATE game_item_instances SET definition_family='Item',definition_production_key=$1,definition_revision_ref=$2,last_transaction_id=$3::uuid WHERE item_instance_id=$4::uuid AND state_revision=$5::text::numeric AND lifecycle=1")
                .bind(target).bind(row.try_get::<String,_>("target_revision")?).bind(&transaction).bind(&item).bind(&revision).execute(&mut **tx).await?.rows_affected()
        } else {
            let changed=sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=$1::uuid WHERE item_instance_id=$2::uuid AND state_revision=$3::text::numeric AND lifecycle=1")
                .bind(&transaction).bind(&item).bind(&revision).execute(&mut **tx).await?.rows_affected();
            let removed=match kind {
                1=>sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=$1::uuid AND stack_ordinal=$2").bind(&item).bind(stack).execute(&mut **tx).await?.rows_affected(),
                2=>sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=$1::uuid AND parent_item_instance_id=$2::uuid AND placement_ordinal=$3::text::numeric").bind(&item).bind(&parent).bind(&ordinal).execute(&mut **tx).await?.rows_affected(),
                _=>sqlx::query("DELETE FROM game_character_equipment_slots WHERE item_instance_id=$1::uuid AND character_id=$2::uuid AND slot=$3").bind(&item).bind(&character).bind(slot).execute(&mut **tx).await?.rows_affected(),
            };
            if removed != 1 {
                return Err(SpellItemError::Rejected("Item timer exact custody removal"));
            }
            changed
        };
        if affected != 1 {
            return Err(SpellItemError::Rejected("Item timer compare-and-change"));
        }
        if kind == 3 {
            let updated=sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=$1::uuid WHERE character_id=$2::uuid AND revision=$3::text::numeric")
                .bind(&transaction).bind(&character).bind(&equipment_revision).execute(&mut **tx).await?;
            if updated.rows_affected() != 1 {
                return Err(SpellItemError::Rejected("Item timer equipment epoch"));
            }
        }
        let envelope:serde_json::Value=sqlx::query_scalar("SELECT jsonb_build_object('receipt',to_jsonb(r),'schedule',to_jsonb(p)) FROM game_spell_item_temporal_receipts r JOIN game_spell_item_temporal_schedules p USING(item_instance_id) WHERE r.transaction_id=$1::uuid")
            .bind(&transaction).fetch_one(&mut **tx).await?;
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|_| SpellItemError::Rejected("Item timer audit encoding"))?;
        sqlx::query("INSERT INTO game_spell_item_temporal_audit(event_id,transaction_id,envelope,envelope_sha256) VALUES($1::uuid,$2::uuid,$3,sha256($3))")
            .bind(&event).bind(&transaction).bind(bytes).execute(&mut **tx).await?;
        changed += 1;
    }
    Ok(changed)
}

/// Every stage is frozen by the complete original source Item policy in the
/// same mint transaction. No timer infers a new definition or duration.
pub(crate) async fn schedule_field_chain_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    request: &SpellItemTransactionRequest,
    definition: &QualifiedItemDefinition,
    item: [u8; 16],
    ordinal: u32,
    occurred_at: i64,
) -> Result<(), SpellItemError> {
    check_transaction(tx, authority).await?;
    schedule_field_chain_rows_in_transaction(
        tx,
        request.transaction_id,
        definition,
        item,
        ordinal,
        occurred_at,
    )
    .await
}
pub(crate) async fn drain_due_field_chains_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    limit: usize,
) -> Result<usize, SpellItemError> {
    if limit > 500 {
        return Err(SpellItemError::Rejected("Item timer batch bound"));
    }
    if limit == 0 {
        return Ok(0);
    }
    check_scope_transaction(tx, authority).await?;
    let world = authority.world_bytes();
    let channel = authority.channel_bytes();
    let mut changed = 0;
    while changed < limit {
        let rows=sqlx::query(r#"SELECT i.item_instance_id::text,i.state_revision::text,i.quantity,p.source_stage,
       p.target_family,p.target_production_key,p.target_revision,g.stack_ordinal,
       e.character_id::text AS bag_character,e.parent_item_instance_id::text AS parent,e.placement_ordinal::text AS ordinal,
       eq.character_id::text AS equipped_character,eq.slot,es.revision::text AS equipment_revision,
       sess.game_session_id::text AS session,sess.character_lease_generation::text AS lease,sess.current_generation::text AS connection,
       game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id
      FROM game_spell_field_temporal_schedules p JOIN game_item_instances i USING(item_instance_id)
      LEFT JOIN game_item_ground_locations g USING(item_instance_id)
      LEFT JOIN game_item_container_entries e USING(item_instance_id)
      LEFT JOIN game_item_container_slots bag ON bag.item_instance_id=e.parent_item_instance_id AND bag.character_id=e.character_id
      LEFT JOIN game_character_equipment_slots eq USING(item_instance_id)
      LEFT JOIN game_character_equipment_state es ON es.character_id=eq.character_id
      LEFT JOIN game_durability_admission_character_guards cg ON cg.character_id=COALESCE(e.character_id,eq.character_id)
      LEFT JOIN game_durability_reconnect_sessions sess ON sess.game_session_id=cg.holder_game_session_id
      WHERE i.lifecycle=1 AND i.quantity=1 AND i.world_id=encode($1,'hex')::uuid
       AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=(p.definition_family,p.definition_production_key,p.definition_revision)
       AND p.expires_at_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
       AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_receipts done WHERE done.item_instance_id=i.item_instance_id AND done.source_stage=p.source_stage)
       AND ((g.world_id=i.world_id AND g.channel_id=encode($2,'hex')::uuid AND g.runtime_scope_ownership_generation=$3::text::numeric)
         OR (((e.world_id=i.world_id AND bag.character_id=e.character_id) OR eq.world_id=i.world_id)
          AND cg.eligible AND cg.lease_generation=sess.character_lease_generation AND sess.session_state=2
          AND sess.character_id=cg.character_id AND sess.runtime_scope_kind=1 AND sess.runtime_scope_world_id=i.world_id
          AND sess.runtime_scope_channel_id=encode($2,'hex')::uuid AND sess.scope_ownership_generation=$3::text::numeric))
      ORDER BY p.expires_at_unix_ms,i.item_instance_id,p.source_stage LIMIT $4 FOR UPDATE OF i,p"#)
        .bind(world.as_slice()).bind(channel.as_slice()).bind(authority.generation().to_string()).bind(1_i64).fetch_all(&mut **tx).await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let item: String = row.try_get("item_instance_id")?;
            let stage: i32 = row.try_get("source_stage")?;
            let transaction: String = row.try_get("transaction_id")?;
            let event: String = row.try_get("event_id")?;
            let revision: String = row.try_get("state_revision")?;
            let parent: Option<String> = row.try_get("parent")?;
            let slot: Option<i16> = row.try_get("slot")?;
            let stack: Option<i64> = row.try_get("stack_ordinal")?;
            let kind: i16 = if stack.is_some() {
                1
            } else if parent.is_some() {
                2
            } else if slot.is_some() {
                3
            } else {
                return Err(SpellItemError::Rejected(
                    "Item timer has no supported exact custody",
                ));
            };
            let character: Option<String> = if kind == 2 {
                row.try_get("bag_character")?
            } else if kind == 3 {
                row.try_get("equipped_character")?
            } else {
                None
            };
            let session: Option<String> = if kind == 1 {
                None
            } else {
                row.try_get("session")?
            };
            let lease: Option<String> = if kind == 1 {
                None
            } else {
                row.try_get("lease")?
            };
            let connection: Option<String> = if kind == 1 {
                None
            } else {
                row.try_get("connection")?
            };
            let ordinal: Option<String> = row.try_get("ordinal")?;
            let equipment_revision: Option<String> = row.try_get("equipment_revision")?;
            sqlx::query("INSERT INTO game_spell_field_temporal_receipts(source_stage,transaction_id,event_id,item_instance_id,world_id,channel_id,ownership_generation,state_revision_before,quantity_before,occurred_at_unix_ms,custody_kind,character_id,game_session_id,character_lease_generation,connection_generation,source_parent,source_ordinal,source_slot,equipment_revision_before,source_stack_ordinal) VALUES($18,$1::uuid,$2::uuid,$3::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7::text::numeric,1,floor(extract(epoch FROM statement_timestamp())*1000)::bigint,$8,$9::uuid,$10::uuid,$11::text::numeric,$12::text::numeric,$13::uuid,$14::text::numeric,$15,$16::text::numeric,$17)")
            .bind(&transaction).bind(&event).bind(&item).bind(world.as_slice()).bind(channel.as_slice()).bind(authority.generation().to_string())
            .bind(&revision).bind(kind).bind(&character).bind(&session).bind(&lease).bind(&connection).bind(&parent).bind(&ordinal).bind(slot).bind(&equipment_revision).bind(stack).bind(stage)
            .execute(&mut **tx).await?;
            let target: Option<String> = row.try_get("target_production_key")?;
            let affected = if let Some(target) = target {
                sqlx::query("UPDATE game_item_instances SET definition_family='Item',definition_production_key=$1,definition_revision_ref=$2,last_transaction_id=$3::uuid WHERE item_instance_id=$4::uuid AND state_revision=$5::text::numeric AND lifecycle=1")
                .bind(target).bind(row.try_get::<String,_>("target_revision")?).bind(&transaction).bind(&item).bind(&revision).execute(&mut **tx).await?.rows_affected()
            } else {
                let changed=sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=$1::uuid WHERE item_instance_id=$2::uuid AND state_revision=$3::text::numeric AND lifecycle=1")
                .bind(&transaction).bind(&item).bind(&revision).execute(&mut **tx).await?.rows_affected();
                let removed=match kind {
                1=>sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=$1::uuid AND stack_ordinal=$2").bind(&item).bind(stack).execute(&mut **tx).await?.rows_affected(),
                2=>sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=$1::uuid AND parent_item_instance_id=$2::uuid AND placement_ordinal=$3::text::numeric").bind(&item).bind(&parent).bind(&ordinal).execute(&mut **tx).await?.rows_affected(),
                _=>sqlx::query("DELETE FROM game_character_equipment_slots WHERE item_instance_id=$1::uuid AND character_id=$2::uuid AND slot=$3").bind(&item).bind(&character).bind(slot).execute(&mut **tx).await?.rows_affected(),
            };
                if removed != 1 {
                    return Err(SpellItemError::Rejected("Item timer exact custody removal"));
                }
                changed
            };
            if affected != 1 {
                return Err(SpellItemError::Rejected("Item timer compare-and-change"));
            }
            if kind == 3 {
                let updated=sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=$1::uuid WHERE character_id=$2::uuid AND revision=$3::text::numeric")
                .bind(&transaction).bind(&character).bind(&equipment_revision).execute(&mut **tx).await?;
                if updated.rows_affected() != 1 {
                    return Err(SpellItemError::Rejected("Item timer equipment epoch"));
                }
            }
            let envelope:serde_json::Value=sqlx::query_scalar("SELECT jsonb_build_object('receipt',to_jsonb(r),'schedule',to_jsonb(p)) FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_schedules p ON p.item_instance_id=r.item_instance_id AND p.source_stage=r.source_stage WHERE r.transaction_id=$1::uuid")
            .bind(&transaction).fetch_one(&mut **tx).await?;
            let bytes = serde_json::to_vec(&envelope)
                .map_err(|_| SpellItemError::Rejected("Item timer audit encoding"))?;
            sqlx::query("INSERT INTO game_spell_field_temporal_audit(event_id,transaction_id,envelope,envelope_sha256) VALUES($1::uuid,$2::uuid,$3,sha256($3))")
            .bind(&event).bind(&transaction).bind(bytes).execute(&mut **tx).await?;
            // Flush this stage's deferred outcome proof before a bounded catch-up
            // transforms the same current instance again. Never loosen a proof to
            // accept a later item state as this stage's immediate successor.
            sqlx::query("SET CONSTRAINTS game_spell_field_temporal_receipt_proven IMMEDIATE")
                .execute(&mut **tx)
                .await?;
            sqlx::query("SET CONSTRAINTS game_spell_field_temporal_receipt_proven DEFERRED")
                .execute(&mut **tx)
                .await?;
            changed += 1;
        }
    }
    Ok(changed)
}

pub(super) async fn schedule_source_scope_field_chain_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    transaction_id: [u8; 16],
    definition: &QualifiedItemDefinition,
    item: [u8; 16],
    ordinal: u32,
    occurred_at: i64,
) -> Result<(), SpellItemError> {
    check_scope_transaction(tx, authority).await?;
    schedule_field_chain_rows_in_transaction(
        tx,
        transaction_id,
        definition,
        item,
        ordinal,
        occurred_at,
    )
    .await
}
async fn schedule_field_chain_rows_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    transaction_id: [u8; 16],
    definition: &QualifiedItemDefinition,
    item: [u8; 16],
    ordinal: u32,
    occurred_at: i64,
) -> Result<(), SpellItemError> {
    if definition.decay_chain.is_empty() {
        return Ok(());
    }
    if definition.decay_chain.len() > 32
        || definition.stack_maximum != 1
        || definition.container_capacity.is_some()
        || !definition.ground_destination
    {
        return Err(SpellItemError::Rejected("field source chain qualification"));
    }
    let mut deadline = occurred_at;
    let mut expected = Some(&definition.definition);
    for (index, stage) in definition.decay_chain.iter().enumerate() {
        if expected != Some(&stage.definition) || stage.duration_millis == 0 {
            return Err(SpellItemError::Rejected("field source chain continuity"));
        }
        deadline = deadline
            .checked_add(i64::from(stage.duration_millis))
            .ok_or(SpellItemError::Rejected(
                "field source chain deadline overflow",
            ))?;
        sqlx::query("INSERT INTO game_spell_field_temporal_schedules(item_instance_id,source_stage,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,target_family,target_production_key,target_revision,content_digest,blocks_movement,blocks_projectile,immovable_block_solid) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)")
            .bind(item.as_slice()).bind(i32::try_from(index+1).map_err(|_|SpellItemError::Rejected("field stage"))?)
            .bind(transaction_id.as_slice()).bind(i32::try_from(ordinal).map_err(|_|SpellItemError::Rejected("field source ordinal"))?)
            .bind(i64::from(stage.duration_millis)).bind(deadline).bind(&stage.definition.family)
            .bind(&stage.definition.production_key).bind(&stage.definition.revision_ref)
            .bind(stage.target.as_ref().map(|t|t.family.as_str())).bind(stage.target.as_ref().map(|t|t.production_key.as_str()))
            .bind(stage.target.as_ref().map(|t|t.revision_ref.as_str())).bind(definition.content_generation_digest.as_slice())
            .bind(stage.blocks_movement).bind(stage.blocks_projectile).bind(stage.immovable_block_solid).execute(&mut **tx).await?;
        expected = stage.target.as_ref();
    }
    // A final permanent transformed definition is allowed only when its
    // qualified source has no further decay; the authoring owner records it.
    Ok(())
}
