//! Entry/tool retirement of a conjured barrier under its actual Ground owner.
use super::spell_item_transaction::{SpellItemError, StandingTileRead};
use sqlx::{Postgres, Row, Transaction};

/// RUNEUSE0-RL-06: the accepted total of live fields in a channel.
pub(super) const CHANNEL_FIELDS_MAX: i64 = 20_000;

/// The sealed tile read owns the Item and Channel locks. Removal is keyed by
/// the unique Item, so a lost acknowledgement cannot remove a replacement.
pub async fn remove_field_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &super::DurabilityRoot,
    read: &StandingTileRead,
    item: [u8; 16],
    tool: Option<[u8; 16]>,
    command: Option<u64>,
    binding: Option<[u8; 32]>,
) -> Result<(), SpellItemError> {
    if tool.is_some() != command.is_some()
        || tool.is_some() != binding.is_some()
        || command == Some(0)
    {
        return Err(SpellItemError::Rejected("field tool command identity"));
    }
    read.check_transaction(tx).await?;
    let row = read
        .items()
        .iter()
        .find(|row| row.item_instance_id == item)
        .ok_or(SpellItemError::Rejected("field is no longer on this tile"))?;
    let allowed = match tool {
        None => matches!(
            row.definition.production_key.as_str(),
            "oteryn:item.tibia.i10181" | "oteryn:item.tibia.i10182"
        ),
        Some(_) => matches!(
            row.definition.production_key.as_str(),
            "oteryn:item.tibia.i2130" | "oteryn:item.tibia.i3635" | "oteryn:item.tibia.i30224"
        ),
    };
    if !allowed || row.field_origin.is_none() {
        return Err(SpellItemError::Rejected("unsupported field removal source"));
    }
    let fence = read.fence();
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(SpellItemError::Rejected("field removal scope"));
    };
    let target = read.target();
    if tool.is_none() {
        let world = super::spell_field_policy::read_world_field_policy_in_transaction(
            tx,
            root,
            fence.runtime_scope,
            fence.scope_ownership_generation.get(),
        )
        .await?
        .ok_or(SpellItemError::Rejected(
            "safe wall World policy unavailable",
        ))?;
        if world.bind(tx).mode() != super::spell_field_policy::FieldWorldType::NoPvp {
            return Err(SpellItemError::Rejected(
                "safe magic wall entry requires Optional PvP",
            ));
        }
    }
    if let Some(tool) = tool {
        let held:Option<bool>=sqlx::query_scalar("SELECT game_field_machete_carried(encode($2,'hex')::uuid,encode($3,'hex')::uuid,item_instance_id) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(tool.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        if held != Some(true) {
            return Err(SpellItemError::Rejected("machete is not currently carried"));
        }
    }
    let result = sqlx::query("INSERT INTO game_field_removals(item_instance_id,transaction_id,event_id,source_transaction_id,character_id,game_session_id,connection_generation,character_lease_generation,world_id,channel_id,ownership_generation,state_revision_before,source_stack_ordinal,spatial_position,map_revision,content_revision,placement_context,tool_item_instance_id,command_id,intent_binding) SELECT i.item_instance_id,game_character_uuid_v7(),game_character_uuid_v7(),i.minted_transaction_id,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric,i.state_revision,g.stack_ordinal,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context,CASE WHEN $13::bytea IS NULL THEN NULL ELSE encode($13,'hex')::uuid END,$14::text::numeric,$15 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1 AND g.world_id=encode($6,'hex')::uuid AND g.channel_id=encode($7,'hex')::uuid AND g.runtime_scope_ownership_generation=$8::text::numeric AND g.spatial_position=$9 AND g.map_revision=$10 AND g.content_revision=$11 AND g.native_room_placement_context=$12 RETURNING transaction_id::text")
        .bind(item.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(&target.spatial_position).bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context)
        .bind(tool.as_ref().map(|value| value.as_slice())).bind(command.map(|id|id.to_string())).bind(binding.as_ref().map(|value|value.as_slice())).fetch_one(&mut **tx).await?;
    let transaction: String = result.try_get("transaction_id")?;
    let changed = sqlx::query("UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1 AND quantity=1 AND state_revision=$3::text::numeric")
        .bind(item.as_slice()).bind(&transaction).bind(row.state_revision.to_string()).execute(&mut **tx).await?.rows_affected();
    let removed = sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid AND stack_ordinal=$2")
        .bind(item.as_slice()).bind(i64::try_from(row.top_down_ordinal).map_err(|_|SpellItemError::Rejected("field stack ordinal"))?).execute(&mut **tx).await?.rows_affected();
    if changed != 1 || removed != 1 {
        return Err(SpellItemError::Rejected("field compare-and-retire"));
    }
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// The current scope owner clears volatile fields of earlier generations, in
/// the same bounded pass as expiry. An old caster never authorizes this write.
pub(super) async fn retire_restarted_fields(
    tx: &mut Transaction<'_, Postgres>,
    authority: &super::spell_item_transaction::SpellItemScopeAuthority,
    limit: usize,
) -> Result<usize, SpellItemError> {
    if limit > 500 {
        return Err(SpellItemError::Rejected("field restart batch bound"));
    }
    super::spell_item_transaction::check_scope_transaction(tx, authority).await?;
    let Some(channel) = authority.channel_bytes() else {
        return Ok(0);
    };
    let rows=sqlx::query("SELECT i.item_instance_id::text,i.state_revision::text,i.minted_transaction_id::text,g.runtime_scope_ownership_generation::text AS generation,g.stack_ordinal,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) JOIN game_spell_item_lines l ON l.item_instance_id=i.item_instance_id AND l.transaction_id=i.minted_transaction_id AND l.operation_kind=1 WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND g.runtime_scope_ownership_generation<$3::text::numeric AND i.lifecycle=1 AND i.quantity=1 AND (l.expires_at_unix_ms IS NOT NULL OR EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p WHERE p.item_instance_id=i.item_instance_id AND p.source_transaction_id=l.transaction_id)) ORDER BY g.stack_ordinal,i.item_instance_id LIMIT $4 FOR UPDATE OF i")
        .bind(authority.world_bytes().as_slice()).bind(channel.as_slice()).bind(authority.generation().to_string()).bind(i64::try_from(limit).map_err(|_|SpellItemError::Rejected("field restart batch bound"))?).fetch_all(&mut **tx).await?;
    for row in &rows {
        let item: String = row.try_get("item_instance_id")?;
        let revision: String = row.try_get("state_revision")?;
        let transaction:String=sqlx::query_scalar("INSERT INTO game_field_restart_retirements(item_instance_id,transaction_id,event_id,source_transaction_id,world_id,channel_id,source_generation,ownership_generation,state_revision_before,source_stack_ordinal,spatial_position,map_revision,content_revision,placement_context) VALUES($1::uuid,game_character_uuid_v7(),game_character_uuid_v7(),$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::text::numeric,$7::text::numeric,$8,$9,$10,$11,$12) RETURNING transaction_id::text")
            .bind(&item).bind(row.try_get::<String,_>("minted_transaction_id")?).bind(authority.world_bytes().as_slice()).bind(channel.as_slice()).bind(row.try_get::<String,_>("generation")?).bind(authority.generation().to_string()).bind(&revision).bind(row.try_get::<i64,_>("stack_ordinal")?).bind(row.try_get::<Vec<u8>,_>("spatial_position")?).bind(row.try_get::<String,_>("map_revision")?).bind(row.try_get::<String,_>("content_revision")?).bind(row.try_get::<Vec<u8>,_>("native_room_placement_context")?).fetch_one(&mut **tx).await?;
        let changed=sqlx::query("UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=$2::uuid WHERE item_instance_id=$1::uuid AND lifecycle=1 AND quantity=1 AND state_revision=$3::text::numeric").bind(&item).bind(&transaction).bind(&revision).execute(&mut **tx).await?.rows_affected();
        let removed=sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=$1::uuid AND stack_ordinal=$2").bind(&item).bind(row.try_get::<i64,_>("stack_ordinal")?).execute(&mut **tx).await?.rows_affected();
        if changed != 1 || removed != 1 {
            return Err(SpellItemError::Rejected("restart field compare-and-retire"));
        }
    }
    Ok(rows.len())
}
