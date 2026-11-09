//! Canonical type-2 cooking proof, operation tag 15.
use super::super::item_mint_audit::{
    self as mint, CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1,
    OneItemOperationV1, OneItemTransactionV1, TransactionEventRefV1, Type2EventTuple,
};
use super::*;
use prost::Message;
use sha2::{Digest, Sha256};

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemCraftV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub proof: Vec<u8>,
}

#[allow(clippy::too_many_arguments)]
pub(in crate::durability) fn encode(
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    recipe: &str,
    content_digest: [u8; 32],
    before: &Value,
    outputs: &[Value],
    transaction: &str,
    event: &str,
    occurred: i64,
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, DurabilityError> {
    encode_with_ingredient(
        fence,
        request,
        recipe,
        content_digest,
        before,
        outputs,
        None,
        transaction,
        event,
        occurred,
        tuple,
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::durability) fn encode_with_ingredient(
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    recipe: &str,
    content_digest: [u8; 32],
    before: &Value,
    outputs: &[Value],
    ingredient: Option<(&Value, i64)>,
    transaction: &str,
    event: &str,
    occurred: i64,
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, DurabilityError> {
    let proof = serde_json::to_vec(&json!({"recipe": recipe, "intent": request.intent,
        "character": fence.character_id.as_bytes(), "before": before, "outputs": outputs,
        "ingredient": ingredient.map(|(before, count)| json!({"before": before, "count": count})),
        "station": request.station.as_ref().map(|s| json!({"family": s.family, "key": s.production_key, "revision": s.revision_ref})),
        "scope_generation": fence.scope_ownership_generation.get(),
        "content_digest": content_digest,
        "gathering": request.gathering.as_ref().map(|g| json!({"placement": g.mutation.placement,"before":g.mutation.before,"after":g.mutation.after,"expected_revision":g.mutation.expected_revision,"reset_epoch":g.scope.reset_epoch,"consume_bait":g.attempt.consumes_bait})),
        "lease_generation": fence.character_lease_generation})).map_err(|_| DurabilityError::InvalidStoredState)?;
    encode_proof(
        fence,
        request.command,
        proof,
        transaction,
        event,
        occurred,
        tuple,
    )
}

/// A map harvest is a craft consequence with a real map operand and no carried debit.
/// It never invents an input ItemInstance to satisfy the item audit envelope.
#[allow(clippy::too_many_arguments)]
pub(in crate::durability) fn encode_map_harvest(
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    proof: Value,
    transaction: &str,
    event: &str,
    occurred: i64,
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, DurabilityError> {
    let proof = serde_json::to_vec(&proof).map_err(|_| DurabilityError::InvalidStoredState)?;
    encode_proof(fence, command, proof, transaction, event, occurred, tuple)
}

#[allow(clippy::too_many_arguments)]
fn encode_proof(
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    proof: Vec<u8>,
    transaction: &str,
    event: &str,
    occurred: i64,
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, DurabilityError> {
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let payload = mint::encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::Craft(OneItemCraftV1 { proof })),
        },
        mint::RL07_PAYLOAD_BYTES_MAX,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let envelope = EventEnvelopeV1 {
        envelope_revision: 1,
        event_id: super::super::item_mint::uuid_text(event)?.to_vec(),
        event_type_id: mint::EVENT_TYPE_ID,
        event_schema_revision: tuple.schema_revision(),
        durability_class: mint::DURABLE_AUDIT,
        privacy_class: mint::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
        occurred_at_unix_ms: occurred,
        world_id: Some(world.as_bytes().to_vec()),
        channel_id: Some(channel.as_bytes().to_vec()),
        game_session_id: Some(fence.game_session_id.as_bytes().to_vec()),
        connection_generation: Some(fence.connection_generation.get()),
        command_id: Some(command.command_id().get()),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: super::super::item_mint::uuid_text(transaction)?.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(CausationRefV1 {
            cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                game_session_id: fence.game_session_id.as_bytes().to_vec(),
                command_id: command.command_id().get(),
            })),
        }),
        server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..EventEnvelopeV1::default()
    };
    let wire = mint::encode_bounded(&envelope, mint::RL07_ENVELOPE_BYTES_MAX)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    mint::decode_common_envelope(&wire).map_err(|_| DurabilityError::InvalidStoredState)?;
    let decoded = OneItemTransactionV1::decode(envelope.payload.as_slice())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if decoded.encode_to_vec() != envelope.payload {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(wire)
}

#[cfg(test)]
mod captured_ingredient_tests {
    use super::*;
    use crate::domain::CharacterId;
    use crate::foundation::{
        ChannelId, CommandId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
        ScopeOwnershipGeneration, WorldId,
    };

    #[test]
    fn m22_audit_binds_captured_target_before_and_exact_debit() -> Result<(), String> {
        let id = super::super::super::item_mint::uuid_text("29020304-0506-7008-800a-0b0c0d0e0f29")
            .map_err(|e| format!("{e:?}"))?;
        let session = GameSessionId::decode(&id).map_err(|e| format!("{e:?}"))?;
        let fence = CurrentCharacterItemFence {
            character_id: CharacterId::from_bytes(id).map_err(|e| format!("{e:?}"))?,
            game_session_id: session,
            connection_generation: ConnectionGeneration::new(1).map_err(|e| format!("{e:?}"))?,
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&id).map_err(|e| format!("{e:?}"))?,
                ChannelId::decode(&id).map_err(|e| format!("{e:?}"))?,
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1)
                .map_err(|e| format!("{e:?}"))?,
        };
        let request = ItemCraftRequest {
            command: CommandRef::new(session, CommandId::new(1).map_err(|e| format!("{e:?}"))?),
            source: id,
            tool: None,
            ingredient: Some(id),
            station: None,
            intent: vec![1],
            gathering: None,
            enchanting: None,
        };
        let before =
            json!({"quantity": 7, "definition_production_key": "oteryn:item.tibia.i10297"});
        let mut wires = Vec::new();
        for debit in [1, 7] {
            let wire = encode_with_ingredient(
                fence,
                &request,
                "oteryn:item-conversion.tainted-souls-antler-talisman",
                [1; 32],
                &json!({"quantity":1}),
                &[],
                Some((&before, debit)),
                "29020304-0506-7008-800a-0b0c0d0e0f29",
                "29020304-0506-7008-800a-0b0c0d0e0f29",
                1,
                Type2EventTuple::V1,
            )
            .map_err(|e| format!("{e:?}"))?;
            let envelope =
                EventEnvelopeV1::decode(wire.as_slice()).map_err(|e| format!("{e:?}"))?;
            let operation = OneItemTransactionV1::decode(envelope.payload.as_slice())
                .map_err(|e| format!("{e:?}"))?;
            let Some(OneItemOperationV1::Craft(craft)) = operation.operation else {
                return Err("craft operation missing".into());
            };
            let proof: Value =
                serde_json::from_slice(&craft.proof).map_err(|e| format!("{e:?}"))?;
            assert_eq!(proof["ingredient"]["before"], before);
            assert_eq!(proof["ingredient"]["count"], debit);
            wires.push(wire);
        }
        assert_ne!(
            wires[0], wires[1],
            "a changed target debit must change the audit envelope"
        );
        Ok(())
    }
}
