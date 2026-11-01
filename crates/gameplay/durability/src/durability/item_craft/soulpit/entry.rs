//! Single-core entry is a zero-output recipe of the existing craft owner.
//! Its deferred proof requires the shared handoff and Soulpit fight in this transaction.

use super::*;
use crate::durability::soulpit::{SoulpitEntryRequest, SoulpitWriteError};

pub(crate) struct EntryDebit {
    pub transaction_id: [u8; 16],
    pub species: String,
}

pub(crate) async fn burn_entry_core(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &SoulpitEntryRequest,
    native: &NativeGameplayState,
) -> Result<EntryDebit, SoulpitWriteError> {
    let (world, channel) = scope_of(&fence).map_err(|_| SoulpitWriteError::AuthorityRejected)?;
    let craft = SoulCoreCraftRequest {
        command: request.command,
        source: request.source,
        target: request.source,
        intent: request.intent.clone(),
        ground: None,
    };
    let inputs = load_inputs(tx, fence, &craft).await?;
    let source = inputs.first().ok_or(SoulpitWriteError::StaleState)?;
    let source_id = native_source_id(native, &source.definition)
        .filter(|id| is_soul_core(*id))
        .ok_or(SoulpitWriteError::Rule(
            crate::domain::soulpit::SoulpitError::InvalidCore,
        ))?;
    if source.quantity == 0 {
        return Err(SoulpitWriteError::Rule(
            crate::domain::soulpit::SoulpitError::InvalidCore,
        ));
    }
    let monster_name = crate::domain::soulpit::soulpit_entry_monster_name(source_id).ok_or(
        SoulpitWriteError::Rule(crate::domain::soulpit::SoulpitError::InvalidMonster),
    )?;
    // Canary compatibility admission alone cannot create a fight absent from the pinned
    // Crystal registry (for example Purple Butterfly). Refuse before any receipt or debit.
    if crate::domain::soulpit::source_monster(&monster_name).is_none() {
        return Err(SoulpitWriteError::Rule(
            crate::domain::soulpit::SoulpitError::InvalidMonster,
        ));
    }
    // Entry uses the donor reverse-variation/name resolver. Native profiles need not carry
    // optional soul_core_item metadata for their literal source name to resolve.
    let species_key = monster_record_for_name(native.creature_profiles(), &monster_name)
        .map(|(key, _)| key.to_owned())
        .ok_or(SoulpitWriteError::Rule(
            crate::domain::soulpit::SoulpitError::InvalidMonster,
        ))?;
    let (transaction, event, occurred): (String, String, i64) = sqlx::query_as(
        "SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint",
    ).fetch_one(&mut ***tx).await?;
    let proof = json!({"inputs":[source.proof(1)],"delivery":{"kind":"handoff","handoff":hex(&request.handoff_id)},
        "channel":hex(channel.as_bytes()),"scope_generation":fence.scope_ownership_generation.get().to_string()});
    let audit_request = ItemCraftRequest {
        command: request.command,
        source: request.source,
        tool: None,
        station: None,
        intent: request.intent.clone(),
        gathering: None,
        enchanting: None,
    };
    let tuple = tx.tuple();
    let envelope = audit::encode(
        fence,
        &audit_request,
        "oteryn:soulpit.entry",
        native.source_digest(),
        &json!({"primary_item":source.before,"soul_core":proof}),
        &[],
        &transaction,
        &event,
        occurred,
        tuple.tuple(),
    )?;
    sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,consume_source,soul_core_proof) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,'oteryn:soulpit.entry',encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10::text::numeric,$11,'[]'::jsonb,$12,sha256($13),true,$14)")
        .bind(&transaction).bind(&event).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice())
        .bind(world.as_bytes().as_slice()).bind(&request.intent).bind(request.source.as_slice())
        .bind(source.parent.as_slice()).bind(&source.ordinal).bind(&source.before).bind(occurred)
        .bind(&envelope).bind(proof).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_item_instances SET quantity=quantity-1,lifecycle=CASE WHEN quantity=1 THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(request.source.as_slice()).bind(&transaction).execute(&mut ***tx).await?;
    if source.quantity == 1 {
        let statement = if source.nested {
            "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid"
        } else {
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid"
        };
        sqlx::query(statement)
            .bind(request.source.as_slice())
            .execute(&mut ***tx)
            .await?;
    }
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
        .bind(request.source.as_slice()).bind(occurred).bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS)
        .bind(&envelope).execute(&mut ***tx).await?;
    Ok(EntryDebit {
        transaction_id: uuid_text(&transaction)?,
        species: species_key,
    })
}
