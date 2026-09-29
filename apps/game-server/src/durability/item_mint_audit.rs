//! Registered DUR-03 native one-item MINT durable-audit encoding: event type 2
//! (`oteryn.events.v1.OneItemTransactionV1`, operation `mint`) carried in the
//! normative ANL-01 `EventEnvelope`. The TRANSFER variant (oneof tag 3) is
//! owned by [`super::item_transfer_audit`]; the MINT gates below reject it.
//!
//! Every bound is a registered hard maximum from
//! `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (decision D50/D51, amended
//! for the B3 merge shapes). Bounds are checked before encode allocation and
//! oversize input is rejected, never truncated. Value lines, transforms,
//! container expansion, item free text and a MINT `CommandRef` have no field
//! here: a payload carrying them rejects as an unknown field in the canonical
//! round trip.

use prost::Message;
use sha2::{Digest, Sha256};

pub const EVENT_TYPE_ID: u32 = 2;
pub const EVENT_SCHEMA_REVISION: u32 = 1;
pub const INTERPRETATION_REVISION: u32 = 1;
pub const RETENTION_PROFILE_ID: &str = "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1";
/// DUR03-AUDIT-RETENTION-S (P90D) in milliseconds; expiry = occurred_at + this.
pub const AUDIT_RETENTION_P90D_MS: i64 = 7_776_000_000;
/// Typed cause of a loot-output MINT (the registered B4 binding value).
pub const LOOT_MINT_TYPED_CAUSE: &str = "loot_mint";
pub const ITEM_LIFECYCLE_LIVE: u32 = 1;

pub(super) const ENVELOPE_REVISION: u32 = 1;
pub(super) const DURABLE_AUDIT: i32 = 2;
pub(super) const RESTRICTED_PLAYER_LINKED: i32 = 3;
const SECURITY_SENSITIVE: i32 = 4;

// Registered hard maxima (RESOURCE_LIMITS_REGISTRY.json, D50/D51; RL-01 and
// RL-06 amended by B3 §4.5 for the two-item merge and top-up shapes, while
// MINT and the one-item TRANSFER shapes stay at 1 item, 1 participant and at
// most 3 work units).
pub const RL01_TOUCHED_ITEM_INSTANCES_MAX: u64 = 2;
pub const RL02_LOCATION_CUSTODY_LINES_MAX: u64 = 2;
pub const RL03_VALUE_LINES_MAX: u64 = 0;
pub const RL04_TRANSFORM_LINES_MAX: u64 = 0;
pub const RL05_CONTAINER_EXPANSION_MAX: u64 = 0;
pub const RL06_PARTICIPANTS_MAX: u64 = 2;
pub const RL06_EFFECT_WORK_UNITS_MAX: u64 = 6;
pub const RL07_EVENTS_MAX: u64 = 1;
pub const RL07_ENVELOPE_BYTES_MAX: usize = 9_216;
pub const RL07_PAYLOAD_BYTES_MAX: usize = 7_936;
pub const RL07_CONTENT_KEY_BYTES_MAX: usize = 512;
pub const RL07_TECHNICAL_FIELD_BYTES_MAX: usize = 128;
pub const RL07_UUID_BYTES: usize = 16;
pub const RL07_PAYLOAD_SHA256_BYTES: usize = 32;
pub const RL08_RETRY_WORK_UNITS_MAX: u8 = 3;
// Inherited ANL-01 ceilings; the lower DUR-03 rows apply conjunctively.
const ANL_ENVELOPE_STRING_MAX: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditError {
    /// A malformed, oversize or unsupported field (registry `INVALID_INPUT`).
    InvalidInput,
    /// A registered count or byte ceiling was exceeded (`CAPACITY_EXCEEDED`).
    CapacityExceeded,
}

// ---- Payload: docs/contracts/game-events/v1/native_one_item_transaction.proto

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemTypedDefinitionRevisionV1 {
    #[prost(string, tag = "1")]
    pub family: String,
    #[prost(string, tag = "2")]
    pub production_key: String,
    #[prost(string, tag = "3")]
    pub revision_ref: String,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemStateV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub item_instance_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub world_id: Vec<u8>,
    #[prost(message, optional, tag = "3")]
    pub definition: Option<OneItemTypedDefinitionRevisionV1>,
    #[prost(uint32, tag = "4")]
    pub quantity: u32,
    #[prost(uint32, tag = "5")]
    pub lifecycle: u32,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemGroundV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub channel_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub spatial_position: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub corpse_ref: Vec<u8>,
    #[prost(string, tag = "5")]
    pub map_revision: String,
    #[prost(string, tag = "6")]
    pub content_revision: String,
    #[prost(bytes = "vec", tag = "7")]
    pub native_room_placement_context: Vec<u8>,
    #[prost(uint64, tag = "8")]
    pub runtime_scope_ownership_generation: u64,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct CreatureDeathOccurrenceRefV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub channel_id: Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub scope_ownership_generation: u64,
    #[prost(uint32, tag = "4")]
    pub actor_local_id: u32,
    #[prost(uint64, tag = "5")]
    pub actor_local_generation: u64,
}

/// MINT provenance. Field 9 (`command_ref`, TRANSFER only) is absent: a
/// server-originated MINT invents no player command.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemProvenanceV1 {
    #[prost(string, tag = "1")]
    pub typed_cause: String,
    #[prost(message, optional, tag = "2")]
    pub death_occurrence: Option<CreatureDeathOccurrenceRefV1>,
    #[prost(message, optional, tag = "3")]
    pub loot_table: Option<OneItemTypedDefinitionRevisionV1>,
    #[prost(string, tag = "4")]
    pub loot_purpose_key: String,
    #[prost(uint32, tag = "5")]
    pub draw_ordinal: u32,
    #[prost(string, tag = "6")]
    pub content_revision: String,
    #[prost(string, tag = "7")]
    pub ruleset_revision: String,
    #[prost(string, tag = "8")]
    pub sim_revision: String,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemMintV1 {
    #[prost(message, optional, tag = "1")]
    pub after: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub destination: Option<OneItemGroundV1>,
    #[prost(message, optional, tag = "3")]
    pub source: Option<OneItemProvenanceV1>,
    #[prost(bool, tag = "4")]
    pub before_semantically_absent: bool,
}

/// `mint` (tag 2), the B3 `transfer` (tag 3, [`super::item_transfer_audit`])
/// and the CHEST-1 `reward_claim_mint` (tag 4,
/// [`super::reward_claim_mint_audit`]).
#[derive(Clone, PartialEq, Eq, prost::Oneof)]
pub enum OneItemOperationV1 {
    #[prost(message, tag = "2")]
    Mint(OneItemMintV1),
    #[prost(message, tag = "3")]
    Transfer(super::item_transfer_audit::OneItemTransferV1),
    #[prost(message, tag = "4")]
    RewardClaimMint(super::reward_claim_mint_audit::OneItemRewardClaimMintV1),
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemTransactionV1 {
    #[prost(uint32, tag = "1")]
    pub interpretation_revision: u32,
    #[prost(oneof = "OneItemOperationV1", tags = "2, 3, 4")]
    pub operation: Option<OneItemOperationV1>,
}

// ---- Normative ANL-01 EventEnvelope: docs/contracts/game-events/v1/foundation.proto

#[derive(Clone, PartialEq, Eq, Message)]
pub struct RuntimeOrderRefV1 {
    #[prost(uint64, tag = "1")]
    pub scope_ownership_generation: u64,
    #[prost(uint64, tag = "2")]
    pub runtime_execution_ordinal: u64,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct TransactionEventRefV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub transaction_id: Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub ordinal: u32,
    #[prost(uint32, tag = "3")]
    pub count: u32,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct EnvelopeCommandRefV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub game_session_id: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub command_id: u64,
}

#[derive(Clone, PartialEq, Eq, prost::Oneof)]
pub enum CausationV1 {
    #[prost(bytes = "vec", tag = "1")]
    EventId(Vec<u8>),
    #[prost(message, tag = "2")]
    Command(EnvelopeCommandRefV1),
    #[prost(bytes = "vec", tag = "3")]
    OperationId(Vec<u8>),
    #[prost(bytes = "vec", tag = "4")]
    TransactionId(Vec<u8>),
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct CausationRefV1 {
    #[prost(oneof = "CausationV1", tags = "1, 2, 3, 4")]
    pub cause: Option<CausationV1>,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct AnalyticsActorRefV1 {
    #[prost(string, tag = "1")]
    pub identity_domain: String,
    #[prost(uint64, tag = "2")]
    pub identity_epoch: u64,
    #[prost(bytes = "vec", tag = "3")]
    pub analytics_actor_id: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct EventEnvelopeV1 {
    #[prost(uint32, tag = "1")]
    pub envelope_revision: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub event_id: Vec<u8>,
    #[prost(uint32, tag = "3")]
    pub event_type_id: u32,
    #[prost(uint32, tag = "4")]
    pub event_schema_revision: u32,
    // EventDurabilityClass / EventPrivacyClass share the int32 wire form.
    #[prost(int32, tag = "5")]
    pub durability_class: i32,
    #[prost(int32, tag = "6")]
    pub privacy_class: i32,
    #[prost(string, tag = "7")]
    pub retention_profile_id: String,
    #[prost(int64, tag = "8")]
    pub occurred_at_unix_ms: i64,
    #[prost(bytes = "vec", optional, tag = "9")]
    pub world_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "10")]
    pub channel_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "11")]
    pub instance_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "12")]
    pub node_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "13")]
    pub game_session_id: Option<Vec<u8>>,
    #[prost(uint64, optional, tag = "14")]
    pub connection_generation: Option<u64>,
    #[prost(message, optional, tag = "15")]
    pub runtime_order: Option<RuntimeOrderRefV1>,
    #[prost(uint64, optional, tag = "16")]
    pub command_id: Option<u64>,
    #[prost(bytes = "vec", optional, tag = "17")]
    pub operation_id: Option<Vec<u8>>,
    #[prost(message, optional, tag = "18")]
    pub transaction_event: Option<TransactionEventRefV1>,
    #[prost(bytes = "vec", optional, tag = "19")]
    pub correlation_id: Option<Vec<u8>>,
    #[prost(message, optional, tag = "20")]
    pub causation: Option<CausationRefV1>,
    #[prost(message, optional, tag = "21")]
    pub analytics_actor: Option<AnalyticsActorRefV1>,
    #[prost(uint32, optional, tag = "22")]
    pub protocol_major: Option<u32>,
    #[prost(string, optional, tag = "23")]
    pub ruleset_revision: Option<String>,
    #[prost(string, optional, tag = "24")]
    pub content_revision: Option<String>,
    #[prost(string, tag = "25")]
    pub server_build_id: String,
    #[prost(bytes = "vec", tag = "26")]
    pub payload: Vec<u8>,
    #[prost(bytes = "vec", tag = "27")]
    pub payload_sha256: Vec<u8>,
}

// ---- Registered per-field bounds (RL-07)

/// A content key or revision: 1..=512 UTF-8 bytes, no control characters.
pub fn check_content_key(value: &str) -> Result<(), AuditError> {
    check_text(value, RL07_CONTENT_KEY_BYTES_MAX)
}

/// A bounded technical text field such as a definition family: 1..=128 bytes.
pub fn check_technical_text(value: &str) -> Result<(), AuditError> {
    check_text(value, RL07_TECHNICAL_FIELD_BYTES_MAX)
}

/// A bounded technical byte field such as a spatial position: 1..=128 bytes.
pub fn check_technical_bytes(value: &[u8]) -> Result<(), AuditError> {
    if value.is_empty() || value.len() > RL07_TECHNICAL_FIELD_BYTES_MAX {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

fn check_text(value: &str, max: usize) -> Result<(), AuditError> {
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

/// Exactly 16 bytes of a non-nil RFC 9562 UUIDv7.
pub fn check_uuid_v7(value: &[u8]) -> Result<(), AuditError> {
    if value.len() != RL07_UUID_BYTES
        || value.iter().all(|byte| *byte == 0)
        || value[6] >> 4 != 7
        || value[8] & 0xc0 != 0x80
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

pub(super) fn check_definition(
    value: Option<&OneItemTypedDefinitionRevisionV1>,
) -> Result<(), AuditError> {
    let value = value.ok_or(AuditError::InvalidInput)?;
    check_technical_text(&value.family)?;
    check_content_key(&value.production_key)?;
    check_content_key(&value.revision_ref)
}

/// Complete closed MINT shape and every registered per-field bound.
pub fn check_mint(value: &OneItemMintV1) -> Result<(), AuditError> {
    let after = value.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let ground = value.destination.as_ref().ok_or(AuditError::InvalidInput)?;
    let source = value.source.as_ref().ok_or(AuditError::InvalidInput)?;
    let death = source
        .death_occurrence
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    check_uuid_v7(&after.item_instance_id)?;
    check_uuid_v7(&after.world_id)?;
    check_definition(after.definition.as_ref())?;
    check_uuid_v7(&ground.world_id)?;
    check_uuid_v7(&ground.channel_id)?;
    check_technical_bytes(&ground.spatial_position)?;
    check_technical_bytes(&ground.corpse_ref)?;
    check_content_key(&ground.map_revision)?;
    check_content_key(&ground.content_revision)?;
    check_technical_bytes(&ground.native_room_placement_context)?;
    check_technical_text(&source.typed_cause)?;
    check_uuid_v7(&death.world_id)?;
    check_uuid_v7(&death.channel_id)?;
    check_definition(source.loot_table.as_ref())?;
    check_content_key(&source.loot_purpose_key)?;
    check_content_key(&source.content_revision)?;
    check_content_key(&source.ruleset_revision)?;
    check_content_key(&source.sim_revision)?;
    // One live item in the death's own Ground scope and generation, with
    // explicit semantic absence before it.
    if !value.before_semantically_absent
        || after.quantity == 0
        || after.lifecycle != ITEM_LIFECYCLE_LIVE
        || source.typed_cause != LOOT_MINT_TYPED_CAUSE
        || after.world_id != ground.world_id
        || death.world_id != ground.world_id
        || death.channel_id != ground.channel_id
        || death.scope_ownership_generation == 0
        || death.actor_local_generation == 0
        || ground.runtime_scope_ownership_generation != death.scope_ownership_generation
    {
        return Err(AuditError::InvalidInput);
    }
    TransactionResourceUsage::MINT.check()
}

// ---- Registered count rows (RL-01..RL-07 events)

/// Per-logical-transaction counts charged against the registered rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionResourceUsage {
    pub touched_item_instances: u64,
    pub location_custody_lines: u64,
    pub value_lines: u64,
    pub transform_lines: u64,
    pub container_expansion: u64,
    pub participants: u64,
    pub effect_work_units: u64,
    pub events: u64,
}

impl TransactionResourceUsage {
    /// One fresh ItemInstance, one Ground destination line, one participant
    /// whose work is the participant plus its one Ground effect, one event.
    pub const MINT: Self = Self {
        touched_item_instances: 1,
        location_custody_lines: 1,
        value_lines: 0,
        transform_lines: 0,
        container_expansion: 0,
        participants: 1,
        effect_work_units: 2,
        events: 1,
    };

    pub fn check(&self) -> Result<(), AuditError> {
        if self.value_lines > RL03_VALUE_LINES_MAX
            || self.transform_lines > RL04_TRANSFORM_LINES_MAX
            || self.container_expansion > RL05_CONTAINER_EXPANSION_MAX
        {
            return Err(AuditError::InvalidInput);
        }
        if self.touched_item_instances > RL01_TOUCHED_ITEM_INSTANCES_MAX
            || self.location_custody_lines > RL02_LOCATION_CUSTODY_LINES_MAX
            || self.participants > RL06_PARTICIPANTS_MAX
            || self.effect_work_units > RL06_EFFECT_WORK_UNITS_MAX
            || self.events > RL07_EVENTS_MAX
        {
            return Err(AuditError::CapacityExceeded);
        }
        Ok(())
    }
}

// ---- Bounded encode and canonical decode

/// Encodes only after `encoded_len` passed the ceiling; never truncates.
pub fn encode_bounded<M: Message>(message: &M, max: usize) -> Result<Vec<u8>, AuditError> {
    let encoded_len = message.encoded_len();
    if encoded_len > max {
        return Err(AuditError::CapacityExceeded);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(encoded_len)
        .map_err(|_| AuditError::CapacityExceeded)?;
    message
        .encode(&mut bytes)
        .map_err(|_| AuditError::CapacityExceeded)?;
    Ok(bytes)
}

/// RL-07 payload gate: size before decode, canonical round trip, closed MINT
/// shape and per-field bounds.
pub fn decode_payload(wire: &[u8]) -> Result<OneItemMintV1, AuditError> {
    if wire.len() > RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = OneItemTransactionV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire || value.interpretation_revision != INTERPRETATION_REVISION {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::Mint(mint)) = value.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_mint(&mint)?;
    Ok(mint)
}

/// RL-07 envelope gate on the normative EventEnvelope: size before decode,
/// canonical round trip, registered event binding, identity and digest widths,
/// one-event membership and the nested payload gate.
pub fn decode_envelope(wire: &[u8]) -> Result<(EventEnvelopeV1, OneItemMintV1), AuditError> {
    let value = decode_common_envelope(wire)?;
    let mint = decode_payload(&value.payload)?;
    let ground = mint.destination.as_ref().ok_or(AuditError::InvalidInput)?;
    if value.world_id.as_deref() != Some(ground.world_id.as_slice())
        || value.channel_id.as_deref() != Some(ground.channel_id.as_slice())
        || value.game_session_id.is_some()
        || value.command_id.is_some()
        || value.causation.is_some()
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((value, mint))
}

/// Operation-independent RL-07 envelope gate shared by MINT and TRANSFER:
/// size before decode, canonical round trip, registered event binding,
/// identity and digest widths and one-event membership.
pub(super) fn decode_common_envelope(wire: &[u8]) -> Result<EventEnvelopeV1, AuditError> {
    if wire.len() > RL07_ENVELOPE_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = EventEnvelopeV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire {
        return Err(AuditError::InvalidInput);
    }
    let membership = value
        .transaction_event
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    if u64::from(membership.count) > RL07_EVENTS_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    check_uuid_v7(&value.event_id)?;
    check_uuid_v7(&membership.transaction_id)?;
    if value.retention_profile_id.len() > ANL_ENVELOPE_STRING_MAX
        || value.server_build_id.is_empty()
        || value.server_build_id.len() > ANL_ENVELOPE_STRING_MAX
        || value.payload_sha256.len() != RL07_PAYLOAD_SHA256_BYTES
        || value.payload_sha256 != Sha256::digest(&value.payload).as_slice()
        || value.envelope_revision != ENVELOPE_REVISION
        || value.event_type_id != EVENT_TYPE_ID
        || value.event_schema_revision != EVENT_SCHEMA_REVISION
        || value.durability_class != DURABLE_AUDIT
        || !(RESTRICTED_PLAYER_LINKED..=SECURITY_SENSITIVE).contains(&value.privacy_class)
        || value.retention_profile_id != RETENTION_PROFILE_ID
        || value.occurred_at_unix_ms <= 0
        || membership.ordinal != 1
        || membership.count != 1
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(value)
}

/// Envelope identity fixed before the first commit attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MintEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: &'a str,
}

/// Build the exact immutable event bytes for one MINT. The returned envelope
/// is re-decoded through both registered gates before it is returned.
pub fn encode_mint_event(
    identity: MintEventIdentity<'_>,
    mint: OneItemMintV1,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    check_mint(&mint)?;
    let ground = mint.destination.as_ref().ok_or(AuditError::InvalidInput)?;
    let (world_id, channel_id) = (ground.world_id.clone(), ground.channel_id.clone());
    let payload = encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::Mint(mint)),
        },
        RL07_PAYLOAD_BYTES_MAX,
    )?;
    let envelope = EventEnvelopeV1 {
        envelope_revision: ENVELOPE_REVISION,
        event_id: identity.event_id.to_vec(),
        event_type_id: EVENT_TYPE_ID,
        event_schema_revision: EVENT_SCHEMA_REVISION,
        durability_class: DURABLE_AUDIT,
        privacy_class: RESTRICTED_PLAYER_LINKED,
        retention_profile_id: RETENTION_PROFILE_ID.into(),
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
    let wire = encode_bounded(&envelope, RL07_ENVELOPE_BYTES_MAX)?;
    let (decoded, _) = decode_envelope(&wire)?;
    if decoded != envelope {
        return Err(AuditError::InvalidInput);
    }
    Ok(wire)
}

#[cfg(test)]
pub(super) mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use serde_json::Value;

    const RESOURCE_REGISTRY: &str =
        include_str!("../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");
    const EVENT_REGISTRY: &str =
        include_str!("../../../../docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json");

    fn registered(id: &str) -> u64 {
        let registry: Value = serde_json::from_str(RESOURCE_REGISTRY).unwrap();
        let rows: Vec<&Value> = registry["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["id"] == id)
            .collect();
        assert_eq!(rows.len(), 1, "exactly one registry row {id}");
        assert!(
            rows[0]["consumers"]
                .as_array()
                .unwrap()
                .iter()
                .any(|consumer| consumer
                    .as_str()
                    .unwrap()
                    .starts_with("apps/game-server/src/durability/item_mint")),
            "{id} names the stage C runtime consumer"
        );
        rows[0]["hard_maximum"].as_u64().unwrap()
    }

    fn uuid(tag: u8) -> Vec<u8> {
        let mut bytes = vec![0; 16];
        bytes[5] = 1;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = tag;
        bytes
    }

    fn definition(family: &str, key: &str, revision: &str) -> OneItemTypedDefinitionRevisionV1 {
        OneItemTypedDefinitionRevisionV1 {
            family: family.into(),
            production_key: key.into(),
            revision_ref: revision.into(),
        }
    }

    fn mint() -> OneItemMintV1 {
        OneItemMintV1 {
            after: Some(OneItemStateV1 {
                item_instance_id: uuid(9),
                world_id: uuid(1),
                definition: Some(definition("ItemType", "fixture:alpha", "rev-a/1")),
                quantity: 1,
                lifecycle: ITEM_LIFECYCLE_LIVE,
            }),
            destination: Some(OneItemGroundV1 {
                world_id: uuid(1),
                channel_id: uuid(2),
                spatial_position: vec![1, 2, 3],
                corpse_ref: uuid(3),
                map_revision: "map-1".into(),
                content_revision: "content-1".into(),
                native_room_placement_context: uuid(6),
                runtime_scope_ownership_generation: 1,
            }),
            source: Some(OneItemProvenanceV1 {
                typed_cause: LOOT_MINT_TYPED_CAUSE.into(),
                death_occurrence: Some(CreatureDeathOccurrenceRefV1 {
                    world_id: uuid(1),
                    channel_id: uuid(2),
                    scope_ownership_generation: 1,
                    actor_local_id: 7,
                    actor_local_generation: 1,
                }),
                loot_table: Some(definition("LootTable", "fixture:loot", "loot-1")),
                loot_purpose_key: "fixture:purpose.drop".into(),
                draw_ordinal: 1,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            }),
            before_semantically_absent: true,
        }
    }

    fn identity() -> MintEventIdentity<'static> {
        let mut event_id = [0; 16];
        event_id.copy_from_slice(&uuid(30));
        let mut transaction_id = [0; 16];
        transaction_id.copy_from_slice(&uuid(40));
        MintEventIdentity {
            event_id,
            transaction_id,
            occurred_at_unix_ms: 1_790_000_000_000,
            server_build_id: "oteryn-game-server/test",
        }
    }

    // Byte-bound probe of D50 §3.2 (every bounded field at its maximum, every
    // varint at its widest, every optional envelope field present). Not a
    // semantically admissible event.
    fn worst_case() -> (usize, usize) {
        let content = "k".repeat(RL07_CONTENT_KEY_BYTES_MAX);
        let technical = "t".repeat(RL07_TECHNICAL_FIELD_BYTES_MAX);
        let technical_bytes = vec![0xa5; RL07_TECHNICAL_FIELD_BYTES_MAX];
        let typed = definition(&technical, &content, &content);
        let payload = OneItemTransactionV1 {
            interpretation_revision: u32::MAX,
            operation: Some(OneItemOperationV1::Mint(OneItemMintV1 {
                after: Some(OneItemStateV1 {
                    item_instance_id: uuid(0xee),
                    world_id: uuid(0xee),
                    definition: Some(typed.clone()),
                    quantity: u32::MAX,
                    lifecycle: u32::MAX,
                }),
                destination: Some(OneItemGroundV1 {
                    world_id: uuid(0xee),
                    channel_id: uuid(0xee),
                    spatial_position: technical_bytes.clone(),
                    corpse_ref: technical_bytes.clone(),
                    map_revision: content.clone(),
                    content_revision: content.clone(),
                    native_room_placement_context: technical_bytes,
                    runtime_scope_ownership_generation: u64::MAX,
                }),
                source: Some(OneItemProvenanceV1 {
                    typed_cause: technical,
                    death_occurrence: Some(CreatureDeathOccurrenceRefV1 {
                        world_id: uuid(0xee),
                        channel_id: uuid(0xee),
                        scope_ownership_generation: u64::MAX,
                        actor_local_id: u32::MAX,
                        actor_local_generation: u64::MAX,
                    }),
                    loot_table: Some(typed),
                    loot_purpose_key: content.clone(),
                    draw_ordinal: u32::MAX,
                    content_revision: content.clone(),
                    ruleset_revision: content.clone(),
                    sim_revision: content,
                }),
                before_semantically_absent: true,
            })),
        };
        worst_case_envelope(encode_bounded(&payload, RL07_PAYLOAD_BYTES_MAX).unwrap())
    }

    /// Wraps `payload` in the widest EventEnvelope (every optional field
    /// present at its bound); returns (payload bytes, envelope bytes).
    pub(in crate::durability) fn worst_case_envelope(payload: Vec<u8>) -> (usize, usize) {
        let text = "s".repeat(ANL_ENVELOPE_STRING_MAX);
        let id = uuid(0xef);
        let envelope = EventEnvelopeV1 {
            envelope_revision: u32::MAX,
            event_id: id.clone(),
            event_type_id: u32::MAX,
            event_schema_revision: u32::MAX,
            durability_class: i32::MAX,
            privacy_class: i32::MAX,
            retention_profile_id: text.clone(),
            occurred_at_unix_ms: i64::MIN,
            world_id: Some(id.clone()),
            channel_id: Some(id.clone()),
            instance_id: Some(id.clone()),
            node_id: Some(id.clone()),
            game_session_id: Some(id.clone()),
            connection_generation: Some(u64::MAX),
            runtime_order: Some(RuntimeOrderRefV1 {
                scope_ownership_generation: u64::MAX,
                runtime_execution_ordinal: u64::MAX,
            }),
            command_id: Some(u64::MAX),
            operation_id: Some(id.clone()),
            transaction_event: Some(TransactionEventRefV1 {
                transaction_id: id.clone(),
                ordinal: u32::MAX,
                count: u32::MAX,
            }),
            correlation_id: Some(id.clone()),
            causation: Some(CausationRefV1 {
                cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                    game_session_id: id.clone(),
                    command_id: u64::MAX,
                })),
            }),
            analytics_actor: Some(AnalyticsActorRefV1 {
                identity_domain: text.clone(),
                identity_epoch: u64::MAX,
                analytics_actor_id: id,
            }),
            protocol_major: Some(u32::MAX),
            ruleset_revision: Some(text.clone()),
            content_revision: Some(text.clone()),
            server_build_id: text,
            payload_sha256: Sha256::digest(&payload).to_vec(),
            payload: payload.clone(),
        };
        let wire = encode_bounded(&envelope, RL07_ENVELOPE_BYTES_MAX).unwrap();
        (payload.len(), wire.len())
    }

    #[test]
    fn constants_equal_the_registered_rows_and_event_binding() {
        for (row, value) in [
            ("DUR03-RL-01", RL01_TOUCHED_ITEM_INSTANCES_MAX),
            ("DUR03-RL-02", RL02_LOCATION_CUSTODY_LINES_MAX),
            ("DUR03-RL-03", RL03_VALUE_LINES_MAX),
            ("DUR03-RL-04", RL04_TRANSFORM_LINES_MAX),
            ("DUR03-RL-05", RL05_CONTAINER_EXPANSION_MAX),
            ("DUR03-RL-06-PARTICIPANTS", RL06_PARTICIPANTS_MAX),
            ("DUR03-RL-06-EFFECT-WORK-UNITS", RL06_EFFECT_WORK_UNITS_MAX),
            ("DUR03-RL-07-EVENTS", RL07_EVENTS_MAX),
            ("DUR03-RL-07-ENVELOPE-BYTES", RL07_ENVELOPE_BYTES_MAX as u64),
            ("DUR03-RL-07-PAYLOAD-BYTES", RL07_PAYLOAD_BYTES_MAX as u64),
            (
                "DUR03-RL-07-CONTENT-KEY-BYTES",
                RL07_CONTENT_KEY_BYTES_MAX as u64,
            ),
            (
                "DUR03-RL-07-TECHNICAL-FIELD-BYTES",
                RL07_TECHNICAL_FIELD_BYTES_MAX as u64,
            ),
            ("DUR03-RL-07-UUID-BYTES", RL07_UUID_BYTES as u64),
            (
                "DUR03-RL-07-PAYLOAD-SHA256-BYTES",
                RL07_PAYLOAD_SHA256_BYTES as u64,
            ),
            ("DUR03-RL-08", u64::from(RL08_RETRY_WORK_UNITS_MAX)),
            (
                "DUR03-AUDIT-RETENTION-S",
                (AUDIT_RETENTION_P90D_MS / 1_000) as u64,
            ),
        ] {
            assert_eq!(registered(row), value, "{row}");
        }
        let events: Value = serde_json::from_str(EVENT_REGISTRY).unwrap();
        let event = events["event_types"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["id"] == EVENT_TYPE_ID)
            .unwrap();
        assert_eq!(event["name"], "DUR03_NATIVE_ONE_ITEM_TRANSACTION");
        assert_eq!(event["current_schema_revision"], EVENT_SCHEMA_REVISION);
        assert_eq!(event["retention_profile_id"], RETENTION_PROFILE_ID);
        assert_eq!(event["durability_class"], "DURABLE_AUDIT");
        assert_eq!(event["privacy_class_floor"], "RESTRICTED_PLAYER_LINKED");
    }

    #[test]
    fn worst_case_mint_matches_the_decision_and_fits_the_registered_caps() {
        // D50 §3.2 table: MINT payload 6,129 B, envelope 7,167 B.
        assert_eq!(worst_case(), (6_129, 7_167));
    }

    #[test]
    fn mint_event_round_trips_through_both_gates() {
        let wire = encode_mint_event(identity(), mint()).unwrap();
        let (envelope, decoded) = decode_envelope(&wire).unwrap();
        assert_eq!(decoded, mint());
        assert_eq!(envelope.event_id, uuid(30));
        assert_eq!(
            envelope.transaction_event,
            Some(TransactionEventRefV1 {
                transaction_id: uuid(40),
                ordinal: 1,
                count: 1
            })
        );
        assert!(envelope.command_id.is_none() && envelope.causation.is_none());
        // Deterministic: the same frozen inputs give the same exact bytes.
        assert_eq!(encode_mint_event(identity(), mint()).unwrap(), wire);
    }

    #[test]
    fn content_key_and_technical_field_bounds_are_max_and_max_plus_one() {
        assert!(check_content_key(&"k".repeat(512)).is_ok());
        assert_eq!(
            check_content_key(&"k".repeat(513)),
            Err(AuditError::InvalidInput)
        );
        assert!(check_technical_text(&"t".repeat(128)).is_ok());
        assert_eq!(
            check_technical_text(&"t".repeat(129)),
            Err(AuditError::InvalidInput)
        );
        assert!(check_technical_bytes(&[1; 128]).is_ok());
        assert_eq!(
            check_technical_bytes(&[1; 129]),
            Err(AuditError::InvalidInput)
        );
        assert_eq!(check_content_key(""), Err(AuditError::InvalidInput));
        assert_eq!(check_content_key("a\0b"), Err(AuditError::InvalidInput));

        // Every field at its registered maximum encodes; one byte more rejects.
        let mut max = mint();
        let source = max.source.as_mut().unwrap();
        source.loot_purpose_key = "p".repeat(512);
        source.sim_revision = "s".repeat(512);
        source.loot_table.as_mut().unwrap().family = "f".repeat(128);
        let ground = max.destination.as_mut().unwrap();
        ground.spatial_position = vec![7; 128];
        ground.map_revision = "m".repeat(512);
        max.after
            .as_mut()
            .unwrap()
            .definition
            .as_mut()
            .unwrap()
            .production_key = "i".repeat(512);
        assert!(encode_mint_event(identity(), max.clone()).is_ok());
        for mutate in [
            |m: &mut OneItemMintV1| m.source.as_mut().unwrap().loot_purpose_key.push('p'),
            |m: &mut OneItemMintV1| m.source.as_mut().unwrap().sim_revision.push('s'),
            |m: &mut OneItemMintV1| {
                m.source
                    .as_mut()
                    .unwrap()
                    .loot_table
                    .as_mut()
                    .unwrap()
                    .family
                    .push('f');
            },
            |m: &mut OneItemMintV1| m.destination.as_mut().unwrap().spatial_position.push(7),
            |m: &mut OneItemMintV1| m.destination.as_mut().unwrap().map_revision.push('m'),
            |m: &mut OneItemMintV1| {
                m.after
                    .as_mut()
                    .unwrap()
                    .definition
                    .as_mut()
                    .unwrap()
                    .production_key
                    .push('i');
            },
        ] {
            let mut over = max.clone();
            mutate(&mut over);
            assert_eq!(
                encode_mint_event(identity(), over),
                Err(AuditError::InvalidInput)
            );
        }
    }

    #[test]
    fn uuid_and_digest_widths_are_exact() {
        assert!(check_uuid_v7(&uuid(1)).is_ok());
        assert_eq!(check_uuid_v7(&uuid(1)[..15]), Err(AuditError::InvalidInput));
        let mut seventeen = uuid(1);
        seventeen.push(0);
        assert_eq!(check_uuid_v7(&seventeen), Err(AuditError::InvalidInput));

        let wire = encode_mint_event(identity(), mint()).unwrap();
        for width in [31_usize, 33] {
            let mut envelope = EventEnvelopeV1::decode(wire.as_slice()).unwrap();
            envelope.payload_sha256.resize(width, 0);
            assert_eq!(
                decode_envelope(&envelope.encode_to_vec()),
                Err(AuditError::InvalidInput)
            );
        }
    }

    #[test]
    fn payload_and_envelope_size_gates_are_max_and_max_plus_one() {
        // A buffer at the cap passes the size gate (and then fails decoding);
        // one byte more is rejected before any decode.
        assert_eq!(
            decode_payload(&vec![0xff; RL07_PAYLOAD_BYTES_MAX]),
            Err(AuditError::InvalidInput)
        );
        assert_eq!(
            decode_payload(&vec![0xff; RL07_PAYLOAD_BYTES_MAX + 1]),
            Err(AuditError::CapacityExceeded)
        );
        assert_eq!(
            decode_envelope(&vec![0xff; RL07_ENVELOPE_BYTES_MAX]).map(|_| ()),
            Err(AuditError::InvalidInput)
        );
        assert_eq!(
            decode_envelope(&vec![0xff; RL07_ENVELOPE_BYTES_MAX + 1]).map(|_| ()),
            Err(AuditError::CapacityExceeded)
        );
        // The encode gate: exactly the encoded length passes, one byte less rejects.
        let message = OneItemTransactionV1 {
            interpretation_revision: 1,
            operation: Some(OneItemOperationV1::Mint(mint())),
        };
        let len = message.encoded_len();
        assert_eq!(encode_bounded(&message, len).unwrap().len(), len);
        assert_eq!(
            encode_bounded(&message, len - 1),
            Err(AuditError::CapacityExceeded)
        );
    }

    #[test]
    fn count_rows_are_max_and_max_plus_one() {
        let maxima = TransactionResourceUsage {
            touched_item_instances: RL01_TOUCHED_ITEM_INSTANCES_MAX,
            location_custody_lines: RL02_LOCATION_CUSTODY_LINES_MAX,
            value_lines: RL03_VALUE_LINES_MAX,
            transform_lines: RL04_TRANSFORM_LINES_MAX,
            container_expansion: RL05_CONTAINER_EXPANSION_MAX,
            participants: RL06_PARTICIPANTS_MAX,
            effect_work_units: RL06_EFFECT_WORK_UNITS_MAX,
            events: RL07_EVENTS_MAX,
        };
        assert!(maxima.check().is_ok());
        assert!(TransactionResourceUsage::MINT.check().is_ok());
        let capacity: [fn(&mut TransactionResourceUsage); 5] = [
            |u| u.touched_item_instances += 1,
            |u| u.location_custody_lines += 1,
            |u| u.participants += 1,
            |u| u.effect_work_units += 1,
            |u| u.events += 1,
        ];
        for over in capacity {
            let mut usage = maxima;
            over(&mut usage);
            assert_eq!(usage.check(), Err(AuditError::CapacityExceeded));
        }
        let closed: [fn(&mut TransactionResourceUsage); 3] = [
            |u| u.value_lines += 1,
            |u| u.transform_lines += 1,
            |u| u.container_expansion += 1,
        ];
        for over in closed {
            let mut usage = maxima;
            over(&mut usage);
            assert_eq!(usage.check(), Err(AuditError::InvalidInput));
        }
        // RL-07 events on the wire: count=1 accepted, count=2 rejected.
        let wire = encode_mint_event(identity(), mint()).unwrap();
        let mut envelope = EventEnvelopeV1::decode(wire.as_slice()).unwrap();
        envelope.transaction_event.as_mut().unwrap().count = 2;
        assert_eq!(
            decode_envelope(&envelope.encode_to_vec()).map(|_| ()),
            Err(AuditError::CapacityExceeded)
        );
    }

    #[test]
    fn closed_shapes_and_foreign_scope_reject() {
        // TRANSFER (tag 3) is not a known field and fails the canonical gate.
        let mut transfer = OneItemTransactionV1 {
            interpretation_revision: 1,
            operation: None,
        }
        .encode_to_vec();
        transfer.extend_from_slice(&[0x1a, 0x00]);
        assert_eq!(decode_payload(&transfer), Err(AuditError::InvalidInput));

        let mut absent = mint();
        absent.before_semantically_absent = false;
        assert_eq!(
            encode_mint_event(identity(), absent),
            Err(AuditError::InvalidInput)
        );
        let mut other_channel = mint();
        other_channel.destination.as_mut().unwrap().channel_id = uuid(8);
        assert_eq!(
            encode_mint_event(identity(), other_channel),
            Err(AuditError::InvalidInput)
        );
        let mut other_generation = mint();
        other_generation
            .destination
            .as_mut()
            .unwrap()
            .runtime_scope_ownership_generation = 2;
        assert_eq!(
            encode_mint_event(identity(), other_generation),
            Err(AuditError::InvalidInput)
        );
    }
}
