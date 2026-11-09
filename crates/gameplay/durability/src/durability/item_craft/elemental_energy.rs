//! Energy soil's Ground mint shares the carried craft's physical transaction.
use super::*;

pub(super) async fn create(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    native: &NativeGameplayState,
    parent: &str,
    occurred: i64,
) -> Result<(), DurabilityError> {
    let policy = native
        .item_policy("oteryn:item.tibia.i2126", "definition-r1")
        .ok_or(DurabilityError::InvalidStoredState)?;
    if !matches!(
        super::super::native_world_items::source_magic_field(policy),
        Ok(true)
    ) || policy
        .record()
        .attributes
        .field_condition
        .as_ref()
        .and_then(|recipe| recipe.condition_values().ok().flatten())
        .is_none()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let (transaction, event, item): (String, String, String) = sqlx::query_as(
        "SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,game_character_uuid_v7()::text"
    ).fetch_one(&mut ***tx).await?;
    let tuple = tx.tuple();
    let envelope = audit::encode_map_harvest(
        fence,
        request.command,
        json!({"recipe":"oteryn:elemental_energy_field.crystal","craft_transaction":parent,
            "item":item,"key":"oteryn:item.tibia.i2126","revision":"definition-r1","count":1,
            "source":request.source,"content_digest":native.source_digest(),
            "placement":request.gathering.as_ref().map(|g|&g.mutation.placement)}),
        &transaction,
        &event,
        occurred,
        tuple.tuple(),
    )?;
    sqlx::query("INSERT INTO game_elemental_energy_field_receipts(transaction_id,event_id,item_instance_id,craft_transaction_id,scope_generation,envelope) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::text::numeric,$6)")
        .bind(&transaction).bind(&event).bind(&item).bind(parent)
        .bind(fence.scope_ownership_generation.get().to_string()).bind(&envelope)
        .execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT item_instance_id,world_id,'Item','oteryn:item.tibia.i2126','definition-r1',1,1,transaction_id FROM game_elemental_energy_field_receipts WHERE transaction_id=$1::uuid")
        .bind(&transaction).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) SELECT item_instance_id,world_id,channel_id,scope_generation,spatial_position,convert_to('elemental-energy-soil','UTF8'),map_revision,content_revision,placement_context FROM game_elemental_energy_field_receipts WHERE transaction_id=$1::uuid")
        .bind(&transaction).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,$5::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision()))
        .bind(tuple.retention_profile_id()).bind(&item).bind(occurred)
        .bind(super::super::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope)
        .execute(&mut ***tx).await?;
    Ok(())
}
