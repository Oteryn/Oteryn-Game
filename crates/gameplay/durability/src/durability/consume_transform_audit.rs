//! Consume/transform proof in the canonical type-2 conversion envelope.
use super::consume_transform::ConsumeTransformRequest;
use super::item_mint_audit::{
    self as mint, CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1,
    OneItemOperationV1, OneItemTransactionV1, TransactionEventRefV1, Type2EventTuple,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::{DurabilityError, character_authority, item_mint};
use prost::Message;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::item_craft::audit::OneItemCraftV1;

#[allow(clippy::too_many_arguments)]
pub(super) fn action_proof(
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    recipe: &str,
    content_digest: [u8; 32],
    before: &Value,
    before_secondary: Option<&Value>,
    outputs: &[Value],
    consume_source: bool,
    source_count: u32,
    effects: &Value,
    extra_inputs: &[Value],
    source_kind: i32,
    source_location: &Value,
) -> Result<Vec<u8>, DurabilityError> {
    let mut proof = json!({"recipe": recipe, "intent": request.intent,
        "character": fence.character_id.as_bytes(), "before": before, "outputs": outputs,
        "secondary": request.target, "before_secondary": before_secondary,
        "consume_source": consume_source, "effects": effects, "extra_inputs": extra_inputs, "source_kind": source_kind, "source_location": source_location,
        "scope_generation": fence.scope_ownership_generation.get(),
        "content_digest": content_digest,
        "lease_generation": fence.character_lease_generation});
    proof["source_count"] = json!(source_count);
    serde_json::to_vec(&proof).map_err(|_| DurabilityError::InvalidStoredState)
}

pub(super) fn payload_fits(proof: &[u8]) -> bool {
    OneItemTransactionV1 {
        interpretation_revision: mint::INTERPRETATION_REVISION,
        operation: Some(OneItemOperationV1::Craft(OneItemCraftV1 {
            proof: proof.to_vec(),
        })),
    }
    .encoded_len()
        <= mint::RL07_PAYLOAD_BYTES_MAX
}

pub(super) fn compact_proof(transaction: &str, digest: &[u8]) -> Result<Vec<u8>, DurabilityError> {
    if digest.len() != 32 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let digest = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    serde_json::to_vec(&json!({
        "schema": "OTERYN_ITEM_ACTION_RECEIPT_COMMITMENT/v1",
        "encoding": "postgresql-jsonb-text-utf8/v1",
        "transaction_id": transaction,
        "receipt_sha256": digest,
    }))
    .map_err(|_| DurabilityError::InvalidStoredState)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    proof: Vec<u8>,
    transaction: &str,
    event: &str,
    occurred: i64,
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, DurabilityError> {
    let world = fence.runtime_scope.world_id();
    let channel = match fence.runtime_scope {
        crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } => Some(channel_id),
        crate::foundation::RuntimeScopeRefV1::Instance { .. } => None,
    };
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
        event_id: item_mint::uuid_text(event)?.to_vec(),
        event_type_id: mint::EVENT_TYPE_ID,
        event_schema_revision: tuple.schema_revision(),
        durability_class: mint::DURABLE_AUDIT,
        privacy_class: mint::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
        occurred_at_unix_ms: occurred,
        world_id: Some(world.as_bytes().to_vec()),
        channel_id: channel.map(|channel| channel.as_bytes().to_vec()),
        game_session_id: Some(fence.game_session_id.as_bytes().to_vec()),
        connection_generation: Some(fence.connection_generation.get()),
        command_id: Some(request.command.command_id().get()),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: item_mint::uuid_text(transaction)?.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(CausationRefV1 {
            cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                game_session_id: fence.game_session_id.as_bytes().to_vec(),
                command_id: request.command.command_id().get(),
            })),
        }),
        server_build_id: character_authority::SERVER_BUILD_ID.into(),
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
