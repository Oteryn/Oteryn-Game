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
/// The successor tuple of event type 2 (ARCH-BATCH-ROOT-PACKETS-V1 §1.7): revision 2 under
/// `DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2`, whose purpose adds the bank part of a fee.
/// Phase 1 (GOLD-FEE-2) reads and verifies it; every production writer still emits
/// [`EVENT_SCHEMA_REVISION`] and [`RETENTION_PROFILE_ID`] until GOLD-FEE-ACT-2.
pub const EVENT_SCHEMA_REVISION_V2: u32 = 2;
pub const RETENTION_PROFILE_ID_V2: &str = "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2";
/// DUR03-AUDIT-RETENTION-S (P90D) in milliseconds; expiry = occurred_at + this.
pub const AUDIT_RETENTION_P90D_MS: i64 = 7_776_000_000;
/// Typed cause of a loot-output MINT (the registered B4 binding value).
pub const LOOT_MINT_TYPED_CAUSE: &str = "loot_mint";
pub const ITEM_LIFECYCLE_LIVE: u32 = 1;
/// `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` (D3 §4.2): the corpse-loot MINT's
/// placement ordinal is `1..=` this many.
pub const GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX: u64 = 16;

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

/// The admitted `(schema_revision, retention_profile_id)` tuples of event type 2: `(1, V1)` or
/// `(2, V2)`, never a mix (ARCH-BATCH-ROOT-PACKETS-V1 §1.7). Every event is judged by the tuple in
/// its own envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type2EventTuple {
    V1,
    V2,
}

impl Type2EventTuple {
    pub const fn schema_revision(self) -> u32 {
        match self {
            Self::V1 => EVENT_SCHEMA_REVISION,
            Self::V2 => EVENT_SCHEMA_REVISION_V2,
        }
    }

    pub const fn retention_profile_id(self) -> &'static str {
        match self {
            Self::V1 => RETENTION_PROFILE_ID,
            Self::V2 => RETENTION_PROFILE_ID_V2,
        }
    }

    /// The tuple of a stored or received envelope; `None` for any other pair.
    pub fn of(schema_revision: u32, retention_profile_id: &str) -> Option<Self> {
        [Self::V1, Self::V2].into_iter().find(|tuple| {
            tuple.schema_revision() == schema_revision
                && tuple.retention_profile_id() == retention_profile_id
        })
    }
}

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
    /// D3-2 corpse-loot MINT destination: exactly one of `destination`
    /// (Ground) and this is set.
    #[prost(message, optional, tag = "5")]
    pub corpse_container_entry: Option<super::item_transfer_audit::OneItemContainerEntryV1>,
}

/// `mint` (tag 2), the B3 `transfer` (tag 3, [`super::item_transfer_audit`]),
/// the CHEST-1 `reward_claim_mint` (tag 4,
/// [`super::reward_claim_mint_audit`]) and the D3-6 `decay_retire` (tag 5,
/// [`super::item_decay_retire_audit`]), the GOLD-FEE `fee_burn` (tag 6) and the TIMED-RT-1b
/// `timed_expiry` (tag 7, [`super::item_timed_state_audit`]) and the MAP-OVERLAY-1b
/// `map_item_mint` (tag 8, [`super::map_item_mint_audit`]).
#[derive(Clone, PartialEq, Eq, prost::Oneof)]
pub enum OneItemOperationV1 {
    #[prost(message, tag = "2")]
    Mint(OneItemMintV1),
    #[prost(message, tag = "3")]
    Transfer(super::item_transfer_audit::OneItemTransferV1),
    #[prost(message, tag = "4")]
    RewardClaimMint(super::reward_claim_mint_audit::OneItemRewardClaimMintV1),
    #[prost(message, tag = "5")]
    DecayRetire(super::item_decay_retire_audit::OneItemDecayRetireV1),
    #[prost(message, tag = "6")]
    FeeBurn(super::item_fee_burn_audit::OneItemFeeBurnV1),
    #[prost(message, tag = "7")]
    TimedExpiry(super::item_timed_state_audit::OneItemTimedExpiryV1),
    #[prost(message, tag = "8")]
    MapItemMint(super::map_item_mint_audit::OneItemMapItemMintV1),
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemTransactionV1 {
    #[prost(uint32, tag = "1")]
    pub interpretation_revision: u32,
    #[prost(oneof = "OneItemOperationV1", tags = "2, 3, 4, 5, 6, 7, 8")]
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
    let source = value.source.as_ref().ok_or(AuditError::InvalidInput)?;
    let death = source
        .death_occurrence
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    check_uuid_v7(&after.item_instance_id)?;
    check_uuid_v7(&after.world_id)?;
    check_definition(after.definition.as_ref())?;
    check_technical_text(&source.typed_cause)?;
    check_uuid_v7(&death.world_id)?;
    check_uuid_v7(&death.channel_id)?;
    check_definition(source.loot_table.as_ref())?;
    check_content_key(&source.loot_purpose_key)?;
    check_content_key(&source.content_revision)?;
    check_content_key(&source.ruleset_revision)?;
    check_content_key(&source.sim_revision)?;
    // Exactly one destination: the death's Ground scope (with its exact
    // World, Channel and generation) or, for a corpse-loot MINT, the entry of
    // the death's own corpse container.
    match (&value.destination, &value.corpse_container_entry) {
        (Some(ground), None) => {
            check_uuid_v7(&ground.world_id)?;
            check_uuid_v7(&ground.channel_id)?;
            check_technical_bytes(&ground.spatial_position)?;
            check_technical_bytes(&ground.corpse_ref)?;
            check_content_key(&ground.map_revision)?;
            check_content_key(&ground.content_revision)?;
            check_technical_bytes(&ground.native_room_placement_context)?;
            if after.world_id != ground.world_id
                || death.world_id != ground.world_id
                || death.channel_id != ground.channel_id
                || ground.runtime_scope_ownership_generation != death.scope_ownership_generation
            {
                return Err(AuditError::InvalidInput);
            }
        }
        (None, Some(entry)) => {
            check_uuid_v7(&entry.parent_item_instance_id)?;
            if entry.parent_item_instance_id == after.item_instance_id
                || !(1..=GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX).contains(&entry.placement_ordinal)
                || after.world_id != death.world_id
                || source.loot_purpose_key == super::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY
            {
                return Err(AuditError::InvalidInput);
            }
        }
        _ => return Err(AuditError::InvalidInput),
    }
    // One live item with explicit semantic absence before it.
    if !value.before_semantically_absent
        || after.quantity == 0
        || after.lifecycle != ITEM_LIFECYCLE_LIVE
        || source.typed_cause != LOOT_MINT_TYPED_CAUSE
        || death.scope_ownership_generation == 0
        || death.actor_local_generation == 0
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
    let (world_id, channel_id) = mint_scope(&mint)?;
    if value.world_id.as_deref() != Some(world_id.as_slice())
        || value.channel_id.as_deref() != Some(channel_id.as_slice())
        || value.game_session_id.is_some()
        || value.command_id.is_some()
        || value.causation.is_some()
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((value, mint))
}

/// The event's World and Channel: the death's own scope, which a Ground
/// destination repeats exactly ([`check_mint`]) and a corpse-container
/// destination leaves implicit.
fn mint_scope(mint: &OneItemMintV1) -> Result<(Vec<u8>, Vec<u8>), AuditError> {
    let death = mint
        .source
        .as_ref()
        .and_then(|source| source.death_occurrence.as_ref())
        .ok_or(AuditError::InvalidInput)?;
    Ok((death.world_id.clone(), death.channel_id.clone()))
}

/// Operation-independent RL-07 envelope gate shared by MINT and TRANSFER:
/// size before decode, canonical round trip, registered event binding (either
/// admitted [`Type2EventTuple`]), identity and digest widths and one-event
/// membership.
pub(super) fn decode_common_envelope(wire: &[u8]) -> Result<EventEnvelopeV1, AuditError> {
    decode_common_envelope_within(wire, RL07_ENVELOPE_BYTES_MAX)
}

/// [`decode_common_envelope`] under a shape's own registered envelope ceiling.
pub(super) fn decode_common_envelope_within(
    wire: &[u8],
    envelope_bytes_max: usize,
) -> Result<EventEnvelopeV1, AuditError> {
    if wire.len() > envelope_bytes_max {
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
        || Type2EventTuple::of(value.event_schema_revision, &value.retention_profile_id).is_none()
        || value.durability_class != DURABLE_AUDIT
        || !(RESTRICTED_PLAYER_LINKED..=SECURITY_SENSITIVE).contains(&value.privacy_class)
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
    let (world_id, channel_id) = mint_scope(&mint)?;
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

/// Golden type-2 envelopes for the GOLD-FEE-2 compatibility qualification (test builds only).
#[cfg(test)]
pub mod golden {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::{AuditError, EventEnvelopeV1, decode_envelope};

    /// Revision-1 envelopes of every type-2 shape, encoded by the `main` codec before GOLD-FEE-2
    /// (the modules' own round-trip fixtures).
    pub const TYPE2_GOLDEN_V1: &[(&str, &str)] = &[
        (
            "mint",
            concat!(
                "080112100000000000017000800000000000001e18022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000029201160a1000000000000170008000000000",
                "00002810011801ca01176f746572796e2d67616d652d7365727665722f74657374d201ca02080112c5020a4c",
                "0a10000000000001700080000000000000091210000000000001700080000000000000011a220a084974656d",
                "54797065120d666978747572653a616c7068611a077265762d612f312001280112610a100000000000017000",
                "80000000000000011210000000000001700080000000000000021a0301020322100000000000017000800000",
                "00000000032a056d61702d313209636f6e74656e742d313a100000000000017000800000000000000640011a",
                "8f010a096c6f6f745f6d696e74122a0a10000000000001700080000000000000011210000000000001700080",
                "000000000000021801200728011a210a094c6f6f745461626c65120c666978747572653a6c6f6f741a066c6f",
                "6f742d312214666978747572653a707572706f73652e64726f7028013209636f6e74656e742d313a0972756c",
                "657365742d31420573696d2d312001da01205fb4057867e3df1164eaa9d904690ba5eb3da6a97e7728254db0",
                "46493dab4b45",
            ),
        ),
        (
            "corpse_mint",
            concat!(
                "080112100000000000017000800000000000001e18022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000029201160a1000000000000170008000000000",
                "00002810011801ca01176f746572796e2d67616d652d7365727665722f74657374d201fd01080112f8010a4c",
                "0a10000000000001700080000000000000091210000000000001700080000000000000011a220a084974656d",
                "54797065120d666978747572653a616c7068611a077265762d612f31200128011a8f010a096c6f6f745f6d69",
                "6e74122a0a100000000000017000800000000000000112100000000000017000800000000000000218012007",
                "28011a210a094c6f6f745461626c65120c666978747572653a6c6f6f741a066c6f6f742d3122146669787475",
                "72653a707572706f73652e64726f7028013209636f6e74656e742d313a0972756c657365742d31420573696d",
                "2d3120012a140a100000000000017000800000000000000a1003da0120bdae96843b80a5382a60c55dd9a350",
                "27893096f723b72d638191c630b49333fd",
            ),
        ),
        (
            "transfer_ContainerSlot",
            concat!(
                "080112100000000000017000800000000000001e18022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000026a1000000000000170008000000000000032",
                "70018001079201160a100000000000017000800000000000002810011801a2011612140a1000000000000170",
                "0080000000000000321007ca01176f746572796e2d67616d652d7365727665722f74657374d201850308011a",
                "80030a500a10000000000001700080000000000000091210000000000001700080000000000000011a260a04",
                "4974656d120f666978747572653a62332e636f696e1a0d646566696e6974696f6e2d7231201e280112500a10",
                "000000000001700080000000000000091210000000000001700080000000000000011a260a044974656d120f",
                "666978747572653a62332e636f696e1a0d646566696e6974696f6e2d7231201e28011a610a10000000000001",
                "700080000000000000011210000000000001700080000000000000021a030102032210000000000001700080",
                "000000000000032a056d61702d313209636f6e74656e742d313a100000000000017000800000000000000640",
                "01222a0a10000000000001700080000000000000291801221000000000000170008000000000000032280138",
                "012a4b0a1667726f756e645f7069636b75705f7472616e736665723209636f6e74656e742d313a0972756c65",
                "7365742d31420573696d2d314a140a10000000000001700080000000000000321007da01205c41d2c5df0c00",
                "70203a4e564a1b87584861f479446ab4ef826b63f529de47c7",
            ),
        ),
        (
            "transfer_NewEntry",
            concat!(
                "080112100000000000017000800000000000001e18022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000026a1000000000000170008000000000000032",
                "70018001079201160a100000000000017000800000000000002810011801a2011612140a1000000000000170",
                "0080000000000000321007ca01176f746572796e2d67616d652d7365727665722f74657374d201990308011a",
                "94030a500a10000000000001700080000000000000091210000000000001700080000000000000011a260a04",
                "4974656d120f666978747572653a62332e636f696e1a0d646566696e6974696f6e2d7231201e280112500a10",
                "000000000001700080000000000000091210000000000001700080000000000000011a260a044974656d120f",
                "666978747572653a62332e636f696e1a0d646566696e6974696f6e2d7231201e28011a610a10000000000001",
                "700080000000000000011210000000000001700080000000000000021a030102032210000000000001700080",
                "000000000000032a056d61702d313209636f6e74656e742d313a100000000000017000800000000000000640",
                "01223e0a10000000000001700080000000000000291801221000000000000170008000000000000032280132",
                "140a100000000000017000800000000000001410032a4b0a1667726f756e645f7069636b75705f7472616e73",
                "6665723209636f6e74656e742d313a0972756c657365742d31420573696d2d314a140a100000000000017000",
                "80000000000000321007da0120e454b467588a7011e96d7dd5f16468da1915b0604867e74a49ae2a7500acba",
                "f5",
            ),
        ),
        (
            "transfer_FullMerge",
            concat!(
                "080112100000000000017000800000000000001e18022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000026a1000000000000170008000000000000032",
                "70018001079201160a100000000000017000800000000000002810011801a2011612140a1000000000000170",
                "0080000000000000321007ca01176f746572796e2d67616d652d7365727665722f74657374d201af0308011a",
                "aa030a500a10000000000001700080000000000000091210000000000001700080000000000000011a260a04",
                "4974656d120f666978747572653a62332e636f696e1a0d646566696e6974696f6e2d7231201e2801124e0a10",
                "000000000001700080000000000000091210000000000001700080000000000000011a260a044974656d120f",
                "666978747572653a62332e636f696e1a0d646566696e6974696f6e2d723128021a610a100000000000017000",
                "80000000000000011210000000000001700080000000000000021a0301020322100000000000017000800000",
                "00000000032a056d61702d313209636f6e74656e742d313a1000000000000170008000000000000006400122",
                "280a1000000000000170008000000000000029180122100000000000017000800000000000003228012a4b0a",
                "1667726f756e645f7069636b75705f7472616e736665723209636f6e74656e742d313a0972756c657365742d",
                "31420573696d2d314a140a10000000000001700080000000000000321007322c0a1000000000000170008000",
                "00000000000a12140a10000000000001700080000000000000141002183c205ada01201313c9360d517b8ef7",
                "4c193a9f07f6ab1769abf72741090d3aa3263ae573d0f7",
            ),
        ),
        (
            "transfer_TopUp",
            concat!(
                "080112100000000000017000800000000000001e18022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000026a1000000000000170008000000000000032",
                "70018001079201160a100000000000017000800000000000002810011801a2011612140a1000000000000170",
                "0080000000000000321007ca01176f746572796e2d67616d652d7365727665722f74657374d201c70308011a",
                "c2030a500a10000000000001700080000000000000091210000000000001700080000000000000011a260a04",
                "4974656d120f666978747572653a62332e636f696e1a0d646566696e6974696f6e2d7231201e280112500a10",
                "000000000001700080000000000000091210000000000001700080000000000000011a260a044974656d120f",
                "666978747572653a62332e636f696e1a0d646566696e6974696f6e2d7231201428011a610a10000000000001",
                "700080000000000000011210000000000001700080000000000000021a030102032210000000000001700080",
                "000000000000032a056d61702d313209636f6e74656e742d313a100000000000017000800000000000000640",
                "01223e0a10000000000001700080000000000000291801221000000000000170008000000000000032280132",
                "140a100000000000017000800000000000001410032a4b0a1667726f756e645f7069636b75705f7472616e73",
                "6665723209636f6e74656e742d313a0972756c657365742d31420573696d2d314a140a100000000000017000",
                "80000000000000321007322c0a100000000000017000800000000000000a12140a1000000000000170008000",
                "0000000000141002185a2064da01201d5f77161a23651e249f15171d44f998f54b3f09b4d4c2beb5844597c3",
                "304d05",
            ),
        ),
        (
            "transfer_corpse",
            concat!(
                "080112100000000000017000800000000000001e18022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000026a1000000000000170008000000000000032",
                "70018001079201160a100000000000017000800000000000002810011801a2011612140a1000000000000170",
                "0080000000000000321007ca01176f746572796e2d67616d652d7365727665722f74657374d201af0308011a",
                "aa030a500a10000000000001700080000000000000091210000000000001700080000000000000011a260a04",
                "4974656d120f666978747572653a62332e636f696e1a0d646566696e6974696f6e2d7231201e280112500a10",
                "000000000001700080000000000000091210000000000001700080000000000000011a260a044974656d120f",
                "666978747572653a62332e636f696e1a0d646566696e6974696f6e2d7231201e2801223e0a10000000000001",
                "700080000000000000291801221000000000000170008000000000000032280132140a100000000000017000",
                "800000000000001410032a4b0a1667726f756e645f7069636b75705f7472616e736665723209636f6e74656e",
                "742d313a0972756c657365742d31420573696d2d314a140a100000000000017000800000000000003210073a",
                "770a100000000000017000800000000000003c10041a610a1000000000000170008000000000000001121000",
                "0000000001700080000000000000021a030102032210000000000001700080000000000000032a056d61702d",
                "313209636f6e74656e742d313a10000000000001700080000000000000064001da01204a92790e385163ab02",
                "6a5b75a9a559f93e5d0ff73b3d523f0d1ce0669aef1b4e",
            ),
        ),
        (
            "reward",
            concat!(
                "080112100000000000017000800000000000001e18022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d095ffbc314a100000000000017000",
                "80000000000000015210000000000001700080000000000000026a1000000000000170008000000000000032",
                "70018001079201160a100000000000017000800000000000001f10011801a2011612140a1000000000000170",
                "0080000000000000321007ca010a746573742d6275696c64d201970208012292020a530a1000000000000170",
                "0080000000000000091210000000000001700080000000000000011a290a044974656d121266697874757265",
                "3a63686573742e636f696e1a0d646566696e6974696f6e2d7231201e2801123e0a1000000000000170008000",
                "0000000000291801221000000000000170008000000000000032280132140a10000000000001700080000000",
                "0000001410031a790a117265776172645f636c61696d5f6d696e743209636f6e74656e742d313a0972756c65",
                "7365742d31420573696d2d314a140a1000000000000170008000000000000032100752310a0b526577617264",
                "436c61696d1213666978747572653a63686573742e636c61696d1a0d646566696e6974696f6e2d72312001da",
                "0120d6ad7befc668cdb853aaf3028400fe1ad34484037b6a936ca3b5a061e378036d",
            ),
        ),
        (
            "decay_entry",
            concat!(
                "080112100000000000017000800000000000001e18022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f563140e0acc5a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000029201160a1000000000000170008000000000",
                "00002810011801ca01176f746572796e2d67616d652d7365727665722f74657374d201bd0208012ab8020a50",
                "0a10000000000001700080000000000000091210000000000001700080000000000000011a260a084974656d",
                "547970651213666978747572653a636f727073652d6c6f6f741a057265762d3120032801124e0a1000000000",
                "0001700080000000000000091210000000000001700080000000000000011a260a084974656d547970651213",
                "666978747572653a636f727073652d6c6f6f741a057265762d31280222770a10000000000001700080000000",
                "0000003c10041a610a1000000000000170008000000000000001121000000000000170008000000000000002",
                "1a030909002210000000000001700080000000000000032a056d61702d313209636f6e74656e742d313a1000",
                "00000000017000800000000000000640012a190a100000000000017000800000000000003c10e0acc5a28c34",
                "3002da0120a8f69f520cd9716dcb86bfcf37176f20872d12c56e4206d41c37365ea9833d3f",
            ),
        ),
        (
            "decay_corpse",
            concat!(
                "080112100000000000017000800000000000001e18022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f563140e0acc5a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000029201160a1000000000000170008000000000",
                "00002810011801ca01176f746572796e2d67616d652d7365727665722f74657374d201a70208012aa2020a50",
                "0a100000000000017000800000000000003c1210000000000001700080000000000000011a260a084974656d",
                "547970651213666978747572653a636f727073652d6c6f6f741a057265762d3120012801124e0a1000000000",
                "00017000800000000000003c1210000000000001700080000000000000011a260a084974656d547970651213",
                "666978747572653a636f727073652d6c6f6f741a057265762d3128021a610a10000000000001700080000000",
                "000000011210000000000001700080000000000000021a030909002210000000000001700080000000000000",
                "032a056d61702d313209636f6e74656e742d313a100000000000017000800000000000000640012a190a1000",
                "00000000017000800000000000003c10e0acc5a28c343002da0120cd52a081a12bad5e4b355eec0426f64dc2",
                "c65eaadda4cb552575eb395d705bd1",
            ),
        ),
        (
            "fee_partial",
            concat!(
                "080112100000000000017000800000000000005018022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000029201160a1000000000000170008000000000",
                "00005110011801ca01176f746572796e2d67616d652d7365727665722d74657374d201cf03080132ca030a28",
                "0a260a126f746572796e3a636861726d2e776f756e6412100000000000017000800000000000004610231823",
                "22100000000000017000800000000000002928023210000000000001700080000000000000013a1000000000",
                "00017000800000000000000240014a100000000000017000800000000000003252a4010a500a100000000000",
                "017000800000000000000a1210000000000001700080000000000000011a260a044974656d12176f74657279",
                "6e3a6974656d2e74696269612e69333033311a057265762d31201e2801124e0a100000000000017000800000",
                "000000000a1210000000000001700080000000000000011a260a044974656d12176f746572796e3a6974656d",
                "2e74696269612e69333033311a057265762d312802180752a6010a500a100000000000017000800000000000",
                "000b1210000000000001700080000000000000011a260a044974656d12176f746572796e3a6974656d2e7469",
                "6269612e69333033311a057265762d312032280112500a100000000000017000800000000000000b12100000",
                "00000001700080000000000000011a260a044974656d12176f746572796e3a6974656d2e74696269612e6933",
                "3033311a057265762d31202d28011803da0120f9710f46113c5b44dc1293e3668225b221a55ae941f24bcac8",
                "381d1f70e44ade",
            ),
        ),
        (
            "fee_change",
            concat!(
                "080112100000000000017000800000000000005018022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000029201160a1000000000000170008000000000",
                "00005110011801ca01176f746572796e2d67616d652d7365727665722d74657374d201e604080132e1040a28",
                "0a260a126f746572796e3a636861726d2e776f756e6412100000000000017000800000000000004610ae1218",
                "9a4e22100000000000017000800000000000002928023210000000000001700080000000000000013a100000",
                "000000017000800000000000000240014a1000000000000170008000000000000032529c010a4c0a10000000",
                "0000017000800000000000000a1210000000000001700080000000000000011a220a044974656d12176f7465",
                "72796e3a6974656d2e74696269612e69333033311a0172200a2801124a0a1000000000000170008000000000",
                "00000a1210000000000001700080000000000000011a220a044974656d12176f746572796e3a6974656d2e74",
                "696269612e69333033311a017228021801529c010a4c0a100000000000017000800000000000000b12100000",
                "00000001700080000000000000011a220a044974656d12176f746572796e3a6974656d2e74696269612e6933",
                "3034331a017220012801124a0a100000000000017000800000000000000b1210000000000001700080000000",
                "000000011a220a044974656d12176f746572796e3a6974656d2e74696269612e69333034331a017228021802",
                "58ec3b62500a4c0a10000000000001700080000000000000141210000000000001700080000000000000011a",
                "220a044974656d12176f746572796e3a6974656d2e74696269612e69333033351a0172204c2801100362500a",
                "4c0a10000000000001700080000000000000151210000000000001700080000000000000011a220a04497465",
                "6d12176f746572796e3a6974656d2e74696269612e69333033311a0172203c28011004da0120ecb009ecd50c",
                "fbf9c02695c75d550ad10749f4d1510f662614980f3741a8bbdb",
            ),
        ),
        (
            "timed_transform",
            concat!(
                "080112100000000000017000800000000000005018022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000029201160a1000000000000170008000000000",
                "00005110011801ca01176f746572796e2d67616d652d7365727665722d74657374d201e40108013adf010a47",
                "0a10000000000001700080000000000000091210000000000001700080000000000000011a1d0a084974656d",
                "54797065120a736f66742d626f6f74731a057265762d3120012801124c0a1000000000000170008000000000",
                "0000091210000000000001700080000000000000011a220a084974656d54797065120f776f726e2d736f6674",
                "2d626f6f74731a057265762d31200128011a1000000000000170008000000000000003221000000000000170",
                "0080000000000000012a1000000000000170008000000000000002300140014801520610e0d40318045a0218",
                "05da012046b187694a2a6f1b6818b6ca082faf072ad5ec17c241830857ce6a08d5902758",
            ),
        ),
        (
            "timed_burn",
            concat!(
                "080112100000000000017000800000000000005018022001280230033a2944555230335f4f4e455f4954454d",
                "5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a100000000000017000",
                "80000000000000015210000000000001700080000000000000029201160a1000000000000170008000000000",
                "00005110011801ca01176f746572796e2d67616d652d7365727665722d74657374d201e10108013adc010a41",
                "0a10000000000001700080000000000000091210000000000001700080000000000000011a170a084974656d",
                "54797065120472696e671a057265762d3120012801123f0a1000000000000170008000000000000009121000",
                "0000000001700080000000000000011a170a084974656d54797065120472696e671a057265762d3128021a10",
                "000000000001700080000000000000032210000000000001700080000000000000012a100000000000017000",
                "800000000000000230013a140a100000000000017000800000000000000710034802520208055a020805da01",
                "204e361918a5dcd940f44cc273ae7e5752f1f458daf7ebf940fc5bdd81e3089a19",
            ),
        ),
    ];

    pub fn unhex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
            .collect()
    }

    /// The shape's own envelope gate.
    pub fn verify_type2_shape(shape: &str, wire: &[u8]) -> Result<EventEnvelopeV1, AuditError> {
        use super::super::{
            item_decay_retire_audit, item_fee_burn_audit, item_timed_state_audit,
            item_transfer_audit, reward_claim_mint_audit,
        };
        match shape.split('_').next().unwrap() {
            "mint" | "corpse" => decode_envelope(wire).map(|(envelope, _)| envelope),
            "transfer" => {
                item_transfer_audit::decode_transfer_envelope(wire).map(|(envelope, _)| envelope)
            }
            "reward" => reward_claim_mint_audit::decode_reward_claim_mint_envelope(wire)
                .map(|(envelope, _)| envelope),
            "decay" => item_decay_retire_audit::decode_decay_retire_envelope(wire)
                .map(|(envelope, _)| envelope),
            "fee" => {
                item_fee_burn_audit::decode_fee_burn_envelope(wire).map(|(envelope, _)| envelope)
            }
            "timed" => item_timed_state_audit::decode_timed_expiry_envelope(wire)
                .map(|(envelope, _)| envelope),
            other => panic!("unknown shape {other}"),
        }
    }
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
            corpse_container_entry: None,
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
                corpse_container_entry: None,
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

    fn corpse_mint(ordinal: u64) -> OneItemMintV1 {
        let mut value = mint();
        value.destination = None;
        value.corpse_container_entry =
            Some(super::super::item_transfer_audit::OneItemContainerEntryV1 {
                parent_item_instance_id: uuid(10),
                placement_ordinal: ordinal,
            });
        value
    }

    #[test]
    fn corpse_loot_mint_round_trips_in_the_death_scope() {
        for ordinal in [1, GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX] {
            let wire = encode_mint_event(identity(), corpse_mint(ordinal)).unwrap();
            let (envelope, decoded) = decode_envelope(&wire).unwrap();
            assert_eq!(decoded, corpse_mint(ordinal));
            assert!(decoded.destination.is_none());
            // The event scope is the death's own World and Channel.
            assert_eq!(envelope.world_id, Some(uuid(1)));
            assert_eq!(envelope.channel_id, Some(uuid(2)));
            assert_eq!(
                encode_mint_event(identity(), corpse_mint(ordinal)).unwrap(),
                wire
            );
        }
    }

    #[test]
    fn corpse_loot_mint_destination_is_closed() {
        // Ordinal 0 and the 17th entry are outside 1..=16.
        for ordinal in [0, GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX + 1] {
            assert_eq!(
                encode_mint_event(identity(), corpse_mint(ordinal)),
                Err(AuditError::InvalidInput)
            );
        }
        // Exactly one destination: both and neither are refused.
        let mut both = corpse_mint(1);
        both.destination = mint().destination;
        assert_eq!(
            encode_mint_event(identity(), both),
            Err(AuditError::InvalidInput)
        );
        let mut neither = corpse_mint(1);
        neither.corpse_container_entry = None;
        assert_eq!(
            encode_mint_event(identity(), neither),
            Err(AuditError::InvalidInput)
        );
        // The parent is a UUIDv7 other than the item itself.
        let mut bad_parent = corpse_mint(1);
        bad_parent
            .corpse_container_entry
            .as_mut()
            .unwrap()
            .parent_item_instance_id = vec![0; 15];
        assert_eq!(
            encode_mint_event(identity(), bad_parent),
            Err(AuditError::InvalidInput)
        );
        let mut self_parent = corpse_mint(1);
        self_parent
            .corpse_container_entry
            .as_mut()
            .unwrap()
            .parent_item_instance_id = uuid(9);
        assert_eq!(
            encode_mint_event(identity(), self_parent),
            Err(AuditError::InvalidInput)
        );
        // A loot entry never carries the corpse's reserved cause, and stays
        // in its death's World.
        let mut reserved = corpse_mint(1);
        reserved.source.as_mut().unwrap().loot_purpose_key =
            super::super::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY.into();
        assert_eq!(
            encode_mint_event(identity(), reserved),
            Err(AuditError::InvalidInput)
        );
        let mut other_world = corpse_mint(1);
        other_world.after.as_mut().unwrap().world_id = uuid(5);
        assert_eq!(
            encode_mint_event(identity(), other_world),
            Err(AuditError::InvalidInput)
        );
    }

    #[test]
    fn corpse_container_entry_is_additive_field_five() {
        // Wire field 5, length-delimited: the first byte alone names it, and
        // a pre-D3 Ground MINT (field 5 absent) never emits it.
        let only_entry = OneItemMintV1 {
            corpse_container_entry: corpse_mint(3).corpse_container_entry,
            ..OneItemMintV1::default()
        };
        assert_eq!(only_entry.encode_to_vec()[0], 0x2a);
        let ground = mint();
        let mut without = ground.clone();
        without.corpse_container_entry = None;
        assert_eq!(ground.encode_to_vec(), without.encode_to_vec());
        assert_eq!(
            OneItemMintV1::decode(corpse_mint(3).encode_to_vec().as_slice()).unwrap(),
            corpse_mint(3)
        );
    }

    use super::golden::{TYPE2_GOLDEN_V1 as GOLDEN_V1, unhex, verify_type2_shape as verify};

    #[test]
    fn every_revision_one_shape_verifies_unchanged_under_the_new_codec() {
        for (shape, hex) in GOLDEN_V1 {
            let wire = unhex(hex);
            let verified = verify(shape, &wire);
            assert!(verified.is_ok(), "{shape}: {verified:?}");
            let envelope = verified.unwrap();
            assert_eq!(
                Type2EventTuple::of(
                    envelope.event_schema_revision,
                    &envelope.retention_profile_id
                ),
                Some(Type2EventTuple::V1),
                "{shape}"
            );
            assert_eq!(envelope.encode_to_vec(), wire, "{shape} is canonical");
        }
    }

    #[test]
    fn every_shape_verifies_under_v1_or_v2_and_never_a_mixed_tuple() {
        assert_eq!(
            Type2EventTuple::of(1, RETENTION_PROFILE_ID),
            Some(Type2EventTuple::V1)
        );
        assert_eq!(
            Type2EventTuple::of(2, RETENTION_PROFILE_ID_V2),
            Some(Type2EventTuple::V2)
        );
        for (shape, hex) in GOLDEN_V1 {
            let base = EventEnvelopeV1::decode(unhex(hex).as_slice()).unwrap();
            let with = |revision: u32, profile: &str| {
                let mut envelope = base.clone();
                envelope.event_schema_revision = revision;
                envelope.retention_profile_id = profile.into();
                envelope.encode_to_vec()
            };
            let verified = verify(shape, &with(2, RETENTION_PROFILE_ID_V2));
            assert!(verified.is_ok(), "{shape} under (2, V2): {verified:?}");
            let v2 = verified.unwrap();
            assert_eq!(
                Type2EventTuple::of(v2.event_schema_revision, &v2.retention_profile_id),
                Some(Type2EventTuple::V2)
            );
            for (case, revision, profile) in [
                ("(1, V2)", 1, RETENTION_PROFILE_ID_V2),
                ("(2, V1)", 2, RETENTION_PROFILE_ID),
                ("revision 3", 3, RETENTION_PROFILE_ID_V2),
                ("revision 0", 0, RETENTION_PROFILE_ID),
                (
                    "another profile",
                    2,
                    "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V3",
                ),
                ("the bank profile", 1, "ECONOMY_LEDGER_RETENTION_V1"),
            ] {
                assert_eq!(
                    verify(shape, &with(revision, profile)),
                    Err(AuditError::InvalidInput),
                    "{shape} {case}"
                );
            }
        }
    }

    #[test]
    fn the_v2_tuple_is_the_registered_successor_profile() {
        let registry: Value = serde_json::from_str(EVENT_REGISTRY).unwrap();
        let profile = |id: &str| {
            registry["retention_profiles"]
                .as_array()
                .unwrap()
                .iter()
                .find(|profile| profile["id"] == id)
                .cloned()
                .expect("retention profile registered")
        };
        let (v1, v2) = (
            profile(RETENTION_PROFILE_ID),
            profile(RETENTION_PROFILE_ID_V2),
        );
        assert_eq!(v2["privacy_class"], v1["privacy_class"]);
        assert!(v2["policy_revision"].as_u64().unwrap() > v1["policy_revision"].as_u64().unwrap());
        // Phase 1: event type 2 stays bound to (1, V1) until GOLD-FEE-ACT-2.
        let event = registry["event_types"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["id"] == EVENT_TYPE_ID)
            .unwrap();
        assert_eq!(event["current_schema_revision"], EVENT_SCHEMA_REVISION);
        assert_eq!(event["retention_profile_id"], RETENTION_PROFILE_ID);
    }

    #[test]
    fn the_bank_value_line_is_refused_on_every_non_fee_operation() {
        use super::super::item_fee_burn_audit::OneItemFeeBankDebitV1;
        let line = OneItemFeeBankDebitV1 {
            entry_id: uuid(90),
            asset: "gold".into(),
            account_id: uuid(91),
            world_id: uuid(1),
            kind: 5,
            line_class: 3,
            debit_gold_units: 1,
            balance_before_gold_units: 1,
            balance_after_gold_units: 0,
        }
        .encode_to_vec();
        // Field 13 (length-delimited) appended to the MINT message is an unknown field.
        let mut operation = mint().encode_to_vec();
        operation.push(13 << 3 | 2);
        prost::encoding::encode_varint(line.len() as u64, &mut operation);
        operation.extend_from_slice(&line);
        let mut wire = OneItemTransactionV1 {
            interpretation_revision: INTERPRETATION_REVISION,
            operation: None,
        }
        .encode_to_vec();
        wire.push(2 << 3 | 2);
        prost::encoding::encode_varint(operation.len() as u64, &mut wire);
        wire.extend_from_slice(&operation);
        assert_eq!(decode_payload(&wire), Err(AuditError::InvalidInput));
        // The same bytes without field 13 are the canonical MINT.
        let canonical = OneItemTransactionV1 {
            interpretation_revision: INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::Mint(mint())),
        }
        .encode_to_vec();
        assert_eq!(decode_payload(&canonical).unwrap(), mint());
    }

    /// The production part of a source file: everything before its test module.
    fn production(path: &str) -> String {
        let source =
            std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
                .unwrap();
        let cut = source.find("\n#[cfg(test)]\nmod ").unwrap_or(source.len());
        source[..cut].to_owned()
    }

    #[test]
    fn every_type2_producer_still_emits_v1_in_phase_one() {
        // §1.7 phase 1: mint, transfer, reward claim, decay retire and timed expiry bind the
        // (1, V1) constants; only GOLD-FEE-ACT-1 routes a tuple through them.
        for path in [
            "src/durability/item_mint.rs",
            "src/durability/item_transfer.rs",
            "src/durability/reward_claim_mint.rs",
            "src/durability/item_decay_retire.rs",
            "src/durability/item_timed_state.rs",
        ] {
            let source = production(path);
            assert!(source.contains("RETENTION_PROFILE_ID"), "{path}");
            assert!(
                !source.contains("_V2") && !source.contains("Type2EventTuple"),
                "{path} emits another tuple"
            );
        }
        // The fee writer's production entry passes (1, V1); the tuple-taking entry is test-only.
        let fee = production("src/durability/item_fee_burn.rs");
        assert!(fee.contains("burn_fee(connection, fence, request, Type2EventTuple::V1)"));
        assert_eq!(
            fee.matches("burn_fee(connection, fence, request,").count(),
            2
        );
        assert!(
            fee.contains("#[cfg(test)]\n#[allow(dead_code)]")
                && fee.contains("\npub async fn burn_fee_in_transaction_under(")
                && fee.find("#[cfg(test)]\n#[allow(dead_code)]")
                    < fee.find("\npub async fn burn_fee_in_transaction_under("),
            "the (2, V2) fee entry is test-only in phase 1"
        );
        assert_eq!(
            (EVENT_SCHEMA_REVISION, RETENTION_PROFILE_ID),
            (1, "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1")
        );
    }
}
