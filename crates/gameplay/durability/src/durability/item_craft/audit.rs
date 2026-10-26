//! Canonical type-2 cooking proof, operation tag 15.
use super::super::item_mint_audit::{
    self as mint, CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1, OneItemOperationV1, OneItemTransactionV1, TransactionEventRefV1,
    Type2EventTuple,
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
pub(super) fn encode(
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
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let proof = serde_json::to_vec(&json!({"recipe": recipe, "intent": request.intent,
        "character": fence.character_id.as_bytes(), "before": before, "outputs": outputs,
        "station": request.station.as_ref().map(|s| json!({"family": s.family, "key": s.production_key, "revision": s.revision_ref})),
        "scope_generation": fence.scope_ownership_generation.get(),
        "content_digest": content_digest,
        "lease_generation": fence.character_lease_generation})).map_err(|_| DurabilityError::InvalidStoredState)?;
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
        command_id: Some(request.command.command_id().get()),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: super::super::item_mint::uuid_text(transaction)?.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(CausationRefV1 {
            cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                game_session_id: fence.game_session_id.as_bytes().to_vec(),
                command_id: request.command.command_id().get(),
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
