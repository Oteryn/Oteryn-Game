//! WRITE-0 §5.3: one closed, text-free PRESERVE_INSTANCE transform audit.
use super::*;
use crate::durability::item_mint_audit::{self as audit, Type2Transaction};
use crate::durability::item_transfer_audit::OneItemCommandRefV1;
use prost::Message;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemWriteOnceV1 {
    #[prost(message, optional, tag = "1")]
    pub before: Option<audit::OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub after: Option<audit::OneItemStateV1>,
    #[prost(message, optional, tag = "3")]
    pub command: Option<OneItemCommandRefV1>,
    #[prost(bytes = "vec", tag = "4")]
    pub character_id: Vec<u8>,
    #[prost(uint64, tag = "5")]
    pub state_revision_before: u64,
    #[prost(uint64, tag = "6")]
    pub state_revision_after: u64,
    #[prost(uint64, tag = "7")]
    pub scope_ownership_generation: u64,
    #[prost(uint64, tag = "8")]
    pub character_lease_generation: u64,
}

/// Canonical, bounded WRITE-0 payload. It has no field capable of carrying player text.
pub fn decode_event(
    bytes: &[u8],
) -> std::result::Result<(audit::EventEnvelopeV1, OneItemWriteOnceV1), audit::AuditError> {
    let invalid = audit::AuditError::InvalidInput;
    let envelope = audit::decode_common_envelope(bytes)?;
    if envelope.payload.len() > audit::RL07_PAYLOAD_BYTES_MAX {
        return Err(audit::AuditError::CapacityExceeded);
    }
    let payload =
        audit::OneItemTransactionV1::decode(envelope.payload.as_slice()).map_err(|_| invalid)?;
    if payload.encode_to_vec() != envelope.payload
        || payload.interpretation_revision != audit::INTERPRETATION_REVISION
    {
        return Err(invalid);
    }
    let Some(audit::OneItemOperationV1::WriteOnce(operation)) = payload.operation else {
        return Err(invalid);
    };
    let before = operation.before.as_ref().ok_or(invalid)?;
    let after = operation.after.as_ref().ok_or(invalid)?;
    let command = operation.command.as_ref().ok_or(invalid)?;
    audit::check_uuid_v7(&before.item_instance_id)?;
    audit::check_uuid_v7(&before.world_id)?;
    audit::check_uuid_v7(&operation.character_id)?;
    audit::check_uuid_v7(&command.game_session_id)?;
    audit::check_definition(before.definition.as_ref())?;
    audit::check_definition(after.definition.as_ref())?;
    if before.item_instance_id != after.item_instance_id
        || before.world_id != after.world_id
        || before.quantity != 1
        || after.quantity != 1
        || before.lifecycle != audit::ITEM_LIFECYCLE_LIVE
        || before.lifecycle != after.lifecycle
        || before.key_number != after.key_number
        || before.definition == after.definition
        || before
            .definition
            .as_ref()
            .is_none_or(|definition| definition.family != "Item")
        || after
            .definition
            .as_ref()
            .is_none_or(|definition| definition.family != "Item")
        || operation.state_revision_before == 0
        || operation.scope_ownership_generation == 0
        || operation.character_lease_generation == 0
        || operation.state_revision_before.checked_add(1) != Some(operation.state_revision_after)
        || command.command_id == 0
        || envelope.command_id != Some(command.command_id)
        || envelope.game_session_id.as_deref() != Some(command.game_session_id.as_slice())
        || !matches!(envelope.causation.as_ref().and_then(|causation| causation.cause.as_ref()),
            Some(audit::CausationV1::Command(cause)) if cause.game_session_id == command.game_session_id && cause.command_id == command.command_id)
        || envelope.world_id.as_deref() != Some(before.world_id.as_slice())
        || envelope
            .channel_id
            .as_deref()
            .is_none_or(|channel| audit::check_uuid_v7(channel).is_err())
        || envelope.instance_id.is_some()
        || envelope
            .connection_generation
            .is_none_or(|generation| generation == 0)
    {
        return Err(invalid);
    }
    Ok((envelope, operation))
}

impl DurabilityRoot {
    /// Historical definition DATA for reconstructing the original text binding after a seal.
    /// This grants no current item/custody authority; reconciliation checks the full binding.
    pub async fn read_write_once_source(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        command: CommandRef,
        character: CharacterId,
        item: [u8; 16],
    ) -> Result<Option<TypedDefinitionRef>> {
        let record = recovery
            .record_for(self)
            .map_err(|_| ItemTextWriteError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            lock_item_text_command(&mut tx, command).await?;
            let source: Option<(String, String, String)> = sqlx::query_as("SELECT before_item->>'definition_family',before_item->>'definition_production_key',before_item->>'definition_revision_ref' FROM game_item_write_once_transforms WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND character_id=encode($3,'hex')::uuid AND item_instance_id=encode($4,'hex')::uuid")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .bind(character.as_bytes().as_slice()).bind(item.as_slice()).fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(source.map(|(family, production_key, revision_ref)| TypedDefinitionRef { family, production_key, revision_ref }))
        })).await.map_err(Into::into)
    }
}

pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterItemFence,
    request: &ItemTextWriteRequest,
    target: &TypedDefinitionRef,
) -> Result<()> {
    for value in [&target.family, &target.production_key, &target.revision_ref] {
        audit::check_content_key(value).map_err(|_| ItemTextWriteError::NotWritable)?;
    }
    if target.family != "Item" || target == &request.definition {
        return Err(ItemTextWriteError::NotWritable);
    }
    let recent: i64 = sqlx::query_scalar("SELECT count(*) FROM (SELECT 1 FROM game_item_text_write_receipts t WHERE character_id=encode($1,'hex')::uuid AND (game_session_id,command_id)<>(encode($2,'hex')::uuid,$3::text::numeric) AND (changed OR EXISTS(SELECT 1 FROM game_item_write_once_transforms w WHERE w.game_session_id=t.game_session_id AND w.command_id=t.command_id)) AND written_at>clock_timestamp()-interval '1 second' LIMIT 2) r")
        .bind(fence.character_id.as_bytes().as_slice()).bind(request.command.game_session_id().as_bytes().as_slice())
        .bind(request.command.command_id().get().to_string()).fetch_one(&mut ***tx).await?;
    if recent >= 2 {
        return Err(ItemTextWriteError::Exhausted);
    }
    let (world, channel) = scope_of(fence).map_err(|_| ItemTextWriteError::AuthorityRejected)?;
    // Custody and exact definition were locked/rechecked by the ordinary text writer.
    let (before, revision, transaction, event, occurred): (serde_json::Value, String, Vec<u8>, Vec<u8>, i64) =
        sqlx::query_as("SELECT to_jsonb(i),state_revision::text,uuid_send(game_character_uuid_v7()),uuid_send(game_character_uuid_v7()),floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(request.item_instance_id.as_slice()).fetch_one(&mut ***tx).await?;
    let revision: u64 = revision
        .parse()
        .map_err(|_| ItemTextWriteError::Exhausted)?;
    let after_revision = revision
        .checked_add(1)
        .ok_or(ItemTextWriteError::Exhausted)?;
    let state = |definition: &TypedDefinitionRef| audit::OneItemStateV1 {
        item_instance_id: request.item_instance_id.to_vec(),
        world_id: world.as_bytes().to_vec(),
        definition: Some(audit::OneItemTypedDefinitionRevisionV1 {
            family: definition.family.clone(),
            production_key: definition.production_key.clone(),
            revision_ref: definition.revision_ref.clone(),
        }),
        quantity: 1,
        lifecycle: audit::ITEM_LIFECYCLE_LIVE,
        key_number: 0,
    };
    let payload = audit::encode_bounded(
        &audit::OneItemTransactionV1 {
            interpretation_revision: audit::INTERPRETATION_REVISION,
            operation: Some(audit::OneItemOperationV1::WriteOnce(OneItemWriteOnceV1 {
                before: Some(state(&request.definition)),
                after: Some(state(target)),
                command: Some(OneItemCommandRefV1 {
                    game_session_id: request.command.game_session_id().as_bytes().to_vec(),
                    command_id: request.command.command_id().get(),
                }),
                character_id: fence.character_id.as_bytes().to_vec(),
                state_revision_before: revision,
                state_revision_after: after_revision,
                scope_ownership_generation: fence.scope_ownership_generation.get(),
                character_lease_generation: fence.character_lease_generation,
            })),
        },
        audit::RL07_PAYLOAD_BYTES_MAX,
    )
    .map_err(|_| ItemTextWriteError::Rejected)?;
    let tuple = tx.tuple();
    let envelope = audit::EventEnvelopeV1 {
        envelope_revision: audit::ENVELOPE_REVISION,
        event_id: event.clone(),
        event_type_id: audit::EVENT_TYPE_ID,
        event_schema_revision: tuple.schema_revision(),
        durability_class: audit::DURABLE_AUDIT,
        privacy_class: audit::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
        occurred_at_unix_ms: occurred,
        world_id: Some(world.as_bytes().to_vec()),
        channel_id: Some(channel.as_bytes().to_vec()),
        game_session_id: Some(request.command.game_session_id().as_bytes().to_vec()),
        command_id: Some(request.command.command_id().get()),
        causation: Some(audit::CausationRefV1 {
            cause: Some(audit::CausationV1::Command(audit::EnvelopeCommandRefV1 {
                game_session_id: request.command.game_session_id().as_bytes().to_vec(),
                command_id: request.command.command_id().get(),
            })),
        }),
        connection_generation: Some(fence.connection_generation.get()),
        transaction_event: Some(audit::TransactionEventRefV1 {
            transaction_id: transaction.clone(),
            ordinal: 1,
            count: 1,
        }),
        server_build_id: crate::durability::character_authority::SERVER_BUILD_ID.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..Default::default()
    };
    let bytes = audit::encode_bounded(&envelope, audit::RL07_ENVELOPE_BYTES_MAX)
        .map_err(|_| ItemTextWriteError::Rejected)?;
    decode_event(&bytes).map_err(|_| ItemTextWriteError::Rejected)?;
    sqlx::query("INSERT INTO game_item_write_once_transforms(transaction_id,event_id,item_instance_id,game_session_id,command_id,character_id,world_id,before_item,target_key,target_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,$9,$10)")
        .bind(&transaction).bind(&event).bind(request.item_instance_id.as_slice())
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(before)
        .bind(&target.production_key).bind(&target.revision_ref).execute(&mut ***tx).await?;
    let changed = sqlx::query("UPDATE game_item_instances SET definition_production_key=$2,definition_revision_ref=$3,last_transaction_id=encode($4,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid AND state_revision=$5::text::numeric AND lifecycle=1 AND quantity=1")
        .bind(request.item_instance_id.as_slice()).bind(&target.production_key).bind(&target.revision_ref)
        .bind(&transaction).bind(revision.to_string()).execute(&mut ***tx).await?;
    if changed.rows_affected() != 1 {
        return Err(ItemTextWriteError::NotWritable);
    }
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(event).bind(transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
        .bind(request.item_instance_id.as_slice()).bind(occurred).bind(audit::AUDIT_RETENTION_P90D_MS).bind(bytes)
        .execute(&mut ***tx).await?;
    Ok(())
}
