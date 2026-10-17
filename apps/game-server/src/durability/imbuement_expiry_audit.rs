//! IMBUE-FORGE-0 §§4.1,4.3,6: one non-caller `ImbueCause::Expire` state mutation.
//! The private continuation packet proposes operation tag 12, preserving HOUSE tag 11.
//! It deletes one zero-time slot; it never burns/transforms an item, charges a fee or binds
//! a new active revision. Registration supplies no runtime or schema admission.

use super::item_mint_audit::{
    self as mint, AuditError, EventEnvelopeV1, OneItemOperationV1, OneItemStateV1,
    OneItemTransactionV1, TransactionEventRefV1, Type2EventTuple, check_content_key, check_uuid_v7,
    encode_bounded,
};
use prost::Message;
use sha2::{Digest, Sha256};

/// The complete retained application binding. Remaining zero is explicit semantic data even
/// though protobuf omits its default scalar. Unknown fields and noncanonical bytes reject.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemImbuementSlotV1 {
    #[prost(uint32, tag = "1")]
    pub slot: u32,
    #[prost(string, tag = "2")]
    pub imbuement_key: String,
    #[prost(string, tag = "3")]
    pub content_revision: String,
    #[prost(string, tag = "4")]
    pub category: String,
    /// 1 Basic, 2 Intricate, 3 Powerful; these are this audit's closed tier values.
    #[prost(uint32, tag = "5")]
    pub tier: u32,
    #[prost(uint64, tag = "6")]
    pub remaining_ms: u64,
    #[prost(bytes = "vec", tag = "7")]
    pub applying_occurrence: Vec<u8>,
    #[prost(bytes = "vec", tag = "8")]
    pub applying_transaction_id: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemImbuementExpiryV1 {
    /// The live item is preserved exactly; only row_before is removed.
    #[prost(message, optional, tag = "1")]
    pub item: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub row_before: Option<OneItemImbuementSlotV1>,
    #[prost(bytes = "vec", tag = "3")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub channel_id: Vec<u8>,
    #[prost(uint64, tag = "6")]
    pub scope_ownership_generation: u64,
    #[prost(bytes = "vec", tag = "7")]
    pub expiry_occurrence: Vec<u8>,
    /// Observed unchanged, never a CharacterRevision successor.
    #[prost(uint64, tag = "8")]
    pub character_revision: u64,
    #[prost(uint64, tag = "9")]
    pub equipment_revision: u64,
    #[prost(uint32, tag = "10")]
    pub equipment_slot: u32,
}

pub fn check_expired_slot(row: &OneItemImbuementSlotV1) -> Result<(), AuditError> {
    for key in [&row.imbuement_key, &row.content_revision, &row.category] {
        check_content_key(key)?;
        if key.trim().is_empty() {
            return Err(AuditError::InvalidInput);
        }
    }
    check_uuid_v7(&row.applying_occurrence)?;
    check_uuid_v7(&row.applying_transaction_id)?;
    if row.slot > 2 || !(1..=3).contains(&row.tier) || row.remaining_ms != 0 {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

pub fn check_imbuement_expiry(value: &OneItemImbuementExpiryV1) -> Result<(), AuditError> {
    let item = value.item.as_ref().ok_or(AuditError::InvalidInput)?;
    check_uuid_v7(&item.item_instance_id)?;
    check_uuid_v7(&item.world_id)?;
    mint::check_definition(item.definition.as_ref())?;
    mint::check_key_number(item)?;
    check_expired_slot(value.row_before.as_ref().ok_or(AuditError::InvalidInput)?)?;
    for id in [
        &value.character_id,
        &value.world_id,
        &value.channel_id,
        &value.expiry_occurrence,
    ] {
        check_uuid_v7(id)?;
    }
    if item.lifecycle != mint::ITEM_LIFECYCLE_LIVE
        || item.quantity == 0
        || item.world_id != value.world_id
        || value.scope_ownership_generation == 0
        || value.character_revision == 0
        || value.equipment_revision == 0
        || !matches!(value.equipment_slot, 1..=8 | 10)
        || value.expiry_occurrence
            == value
                .row_before
                .as_ref()
                .ok_or(AuditError::InvalidInput)?
                .applying_occurrence
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

pub fn decode_imbuement_expiry_payload(
    wire: &[u8],
) -> Result<OneItemImbuementExpiryV1, AuditError> {
    if wire.len() > mint::RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let transaction = OneItemTransactionV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if transaction.interpretation_revision != mint::INTERPRETATION_REVISION
        || transaction.encode_to_vec() != wire
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::ImbuementExpiry(value)) = transaction.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_imbuement_expiry(&value)?;
    Ok(value)
}

pub fn decode_imbuement_expiry_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, OneItemImbuementExpiryV1), AuditError> {
    let envelope = mint::decode_common_envelope(wire)?;
    let expiry = decode_imbuement_expiry_payload(&envelope.payload)?;
    if envelope.world_id.as_deref() != Some(expiry.world_id.as_slice())
        || envelope.channel_id.as_deref() != Some(expiry.channel_id.as_slice())
        || envelope.game_session_id.is_some()
        || envelope.command_id.is_some()
        || envelope.connection_generation.is_some()
        || envelope.causation.is_some()
        || envelope.instance_id.is_some()
        || envelope.node_id.is_some()
        || envelope.runtime_order.is_some()
        || envelope.operation_id.is_some()
        || envelope.correlation_id.is_some()
        || envelope.analytics_actor.is_some()
        || envelope.protocol_major.is_some()
        || envelope.ruleset_revision.is_some()
        || envelope.content_revision.is_some()
        || envelope.event_id
            == envelope
                .transaction_event
                .as_ref()
                .ok_or(AuditError::InvalidInput)?
                .transaction_id
        || envelope.occurred_at_unix_ms > i64::MAX - mint::AUDIT_RETENTION_P90D_MS
        || expiry.expiry_occurrence
            == expiry
                .row_before
                .as_ref()
                .ok_or(AuditError::InvalidInput)?
                .applying_occurrence
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((envelope, expiry))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImbuementExpiryEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: &'a str,
}

pub fn encode_imbuement_expiry_event(
    identity: ImbuementExpiryEventIdentity<'_>,
    expiry: OneItemImbuementExpiryV1,
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    mint::check_technical_text(identity.server_build_id)?;
    if identity.event_id == identity.transaction_id
        || identity.occurred_at_unix_ms <= 0
        || identity.occurred_at_unix_ms > i64::MAX - mint::AUDIT_RETENTION_P90D_MS
    {
        return Err(AuditError::InvalidInput);
    }
    check_imbuement_expiry(&expiry)?;
    let world_id = expiry.world_id.clone();
    let channel_id = expiry.channel_id.clone();
    let payload = encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::ImbuementExpiry(expiry)),
        },
        mint::RL07_PAYLOAD_BYTES_MAX,
    )?;
    let envelope = EventEnvelopeV1 {
        envelope_revision: mint::ENVELOPE_REVISION,
        event_id: identity.event_id.to_vec(),
        event_type_id: mint::EVENT_TYPE_ID,
        event_schema_revision: tuple.schema_revision(),
        durability_class: mint::DURABLE_AUDIT,
        privacy_class: mint::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
        occurred_at_unix_ms: identity.occurred_at_unix_ms,
        world_id: Some(world_id),
        channel_id: Some(channel_id),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: identity.transaction_id.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        server_build_id: identity.server_build_id.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..EventEnvelopeV1::default()
    };
    let wire = encode_bounded(&envelope, mint::RL07_ENVELOPE_BYTES_MAX)?;
    let (decoded, _) = decode_imbuement_expiry_envelope(&wire)?;
    if decoded != envelope {
        return Err(AuditError::InvalidInput);
    }
    Ok(wire)
}
