//! Consume a bounded floor input and transform a different existing physical Item.
mod expiry;
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::item_mint_audit::Type2Transaction;
pub(in crate::durability) use expiry::drain_due;
use oteryn_gameplay_items::floor_assembly::FloorTransformRecipe;

#[allow(clippy::too_many_arguments)]
pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    permit: &SpellLanePermit,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    native: &NativeGameplayState,
    moved: &LoopMoveReceipt,
    target: &super::floor_assembly::FloorAssemblyTarget,
    ground: &LoopGround,
    recipe: &FloorTransformRecipe,
) -> LoopResult<bool> {
    permit.check_scope(fence.runtime_scope)?;
    let (world, channel) =
        scope_of(fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
    let before: Option<serde_json::Value> = sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'ground',to_jsonb(g)) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND g.spatial_position=$3 AND g.map_revision=$4 AND g.content_revision=$5 AND g.native_room_placement_context=$6 AND g.runtime_scope_ownership_generation=$7::text::numeric AND game_item_loop_drop_ref(g.corpse_ref) AND g.stack_ordinal IS NOT NULL AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1' AND i.definition_production_key=$8 AND i.quantity=1 ORDER BY g.stack_ordinal,i.item_instance_id LIMIT 1 FOR UPDATE OF i NOWAIT")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&ground.spatial_position)
        .bind(&ground.map_revision).bind(&ground.content_revision).bind(&ground.placement_context)
        .bind(fence.scope_ownership_generation.get().to_string()).bind(recipe.target).fetch_optional(&mut ***tx).await?;
    let Some(before) = before else {
        return Ok(false);
    };
    let input: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'ground',to_jsonb(g)) FROM game_item_loop_move_lines l JOIN game_item_instances i USING(item_instance_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE l.transaction_id=encode($1,'hex')::uuid AND l.to_kind=3 AND l.definition_family='Item' AND l.definition_production_key=$2 AND i.lifecycle=1 AND i.definition_production_key=l.definition_production_key AND i.quantity >= $3 AND g.spatial_position=$4 ORDER BY l.line_ordinal LIMIT 1 FOR UPDATE OF i NOWAIT")
        .bind(moved.transaction_id.as_slice()).bind(recipe.input).bind(i64::from(recipe.input_count)).bind(&ground.spatial_position).fetch_one(&mut ***tx).await?;
    let policy = native
        .item_policy(recipe.output, "definition-r1")
        .ok_or(DurabilityError::InvalidStoredState)?;
    let charge_free =
        policy.record().semantics.charges == crate::content::ReferenceItemField::NotApplicable;
    let qualified =
        crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    let decay = qualified.decay.ok_or(DurabilityError::InvalidStoredState)?;
    if !charge_free
        || !qualified.movable
        || !qualified.ground_destination
        || !qualified.inventory_destination
        || qualified.stack_maximum != 1
        || qualified.container_capacity.is_some()
        || decay.duration_millis != recipe.duration_millis
        || decay
            .target
            .as_ref()
            .map(|target| (target.production_key.as_str(), target.revision_ref.as_str()))
            != Some((recipe.decay_target, "definition-r1"))
    {
        return refuse(LoopMoveRefusal::NotSupported);
    }
    let (transaction,event,occurred): (String,String,i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
        .fetch_one(&mut ***tx).await?;
    let occurred = target.source_occurrence(occurred);
    let deadline = occurred
        .checked_add(i64::from(recipe.duration_millis))
        .ok_or(DurabilityError::InvalidStoredState)?;
    let item = before["item"]["item_instance_id"]
        .as_str()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let consumable = input["item"]["item_instance_id"]
        .as_str()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let proof = json!({"recipe":recipe.key,"input":input,"target":before,"output_key":recipe.output,
        "duration_millis":recipe.duration_millis,"expires_at_unix_ms":deadline,"decay_target":recipe.decay_target,
        "move_transaction":moved.transaction_id,"placement":target.placement,"reset_epoch":target.scope.reset_epoch,
        "content_digest":native.source_digest()});
    let tuple = tx.tuple();
    let envelope = crate::durability::item_craft::audit::encode_map_harvest(
        *fence,
        command,
        proof,
        &transaction,
        &event,
        occurred,
        tuple.tuple(),
    )?;
    sqlx::query("SET CONSTRAINTS ALL DEFERRED")
        .execute(&mut ***tx)
        .await?;
    sqlx::query("INSERT INTO game_floor_transform_receipts(transaction_id,event_id,move_transaction_id,recipe,input_before,target_before,placement,reset_epoch,content_digest,occurred_at,expires_at_unix_ms,envelope_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4,$5,$6,$7,$8,$9,$10,$11,sha256($12))")
        .bind(&transaction).bind(&event).bind(moved.transaction_id.as_slice()).bind(recipe.key).bind(&input).bind(&before)
        .bind(&target.placement).bind(target.scope.reset_epoch).bind(native.source_digest().as_slice()).bind(occurred).bind(deadline).bind(&envelope)
        .execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_item_instances SET quantity=quantity-$2,lifecycle=CASE WHEN quantity=$2 THEN 2 ELSE 1 END,last_transaction_id=$3::uuid WHERE item_instance_id=$1::uuid")
        .bind(consumable).bind(i64::from(recipe.input_count)).bind(&transaction).execute(&mut ***tx).await?;
    sqlx::query("DELETE FROM game_item_ground_locations g USING game_item_instances i WHERE g.item_instance_id=$1::uuid AND i.item_instance_id=g.item_instance_id AND i.lifecycle=2")
        .bind(consumable).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_item_instances SET definition_production_key=$2,last_transaction_id=$3::uuid WHERE item_instance_id=$1::uuid")
        .bind(item).bind(recipe.output).bind(&transaction).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,$5::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(item).bind(occurred)
        .bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut ***tx).await?;
    Ok(true)
}
