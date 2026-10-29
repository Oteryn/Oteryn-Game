//! DEATH-DUR-1 additive operation tags 17/18. The local projection keeps older operation
//! encoders closed; its canonical round trip rejects every other operation and unknown field.
use super::{PlayerDeathItemRequest, plan};
use crate::durability::item_mint_audit::{
    self as mint, AuditError, EventEnvelopeV1, OneItemGroundV1, OneItemStateV1, Type2EventTuple,
};
use crate::durability::item_transfer_audit::OneItemContainerEntryV1;
use prost::Message;
use sha2::{Digest, Sha256};

pub const AMULET_SINK: &str = "PlayerDeathAmuletCause";
/// Fixed per-operation rows: items, custody lines, value lines, transforms, expansion,
/// participants, effect work, events, payload bytes, envelope bytes, retry work.
pub const DROP_LIMITS: [u64; 11] = [1, 2, 0, 0, 0, 2, 3, 1, 7936, 9216, 3];
pub const AMULET_LIMITS: [u64; 11] = [1, 1, 0, 0, 0, 2, 3, 1, 7936, 9216, 3];
pub fn check_usage(burn: bool, usage: [u64; 11]) -> Result<(), AuditError> {
    let limits = if burn { AMULET_LIMITS } else { DROP_LIMITS };
    if usage.iter().zip(limits).any(|(used, max)| *used > max) {
        return Err(AuditError::CapacityExceeded);
    }
    Ok(())
}
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemPlayerDeathItemV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub death_occurrence: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub item_instance: Vec<u8>,
    #[prost(message, optional, tag = "3")]
    pub before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "4")]
    pub after: Option<OneItemStateV1>,
    #[prost(bytes = "vec", tag = "5")]
    pub character_id: Vec<u8>,
    #[prost(uint32, tag = "6")]
    pub source_slot: u32,
    #[prost(message, optional, tag = "7")]
    pub source_entry: Option<OneItemContainerEntryV1>,
    #[prost(message, optional, tag = "8")]
    pub ground: Option<OneItemGroundV1>,
    #[prost(string, tag = "9")]
    pub typed_sink: String,
    #[prost(string, tag = "10")]
    pub content_revision: String,
    #[prost(string, tag = "11")]
    pub ruleset_revision: String,
    #[prost(string, tag = "12")]
    pub sim_revision: String,
    #[prost(uint64, tag = "13")]
    pub session_generation: u64,
    #[prost(bytes = "vec", tag = "14")]
    pub game_session_id: Vec<u8>,
    #[prost(uint64, tag = "15")]
    pub lease_generation: u64,
}
#[derive(Clone, PartialEq, Eq, prost::Oneof)]
enum Operation {
    #[prost(message, tag = "17")]
    Drop(OneItemPlayerDeathItemV1),
    #[prost(message, tag = "18")]
    Amulet(OneItemPlayerDeathItemV1),
}
#[derive(Clone, PartialEq, Eq, Message)]
struct Payload {
    #[prost(uint32, tag = "1")]
    interpretation_revision: u32,
    #[prost(oneof = "Operation", tags = "17,18")]
    operation: Option<Operation>,
}
fn check(burn: bool, value: &OneItemPlayerDeathItemV1) -> Result<(), AuditError> {
    for id in [
        &value.death_occurrence,
        &value.item_instance,
        &value.character_id,
        &value.game_session_id,
    ] {
        mint::check_uuid_v7(id)?;
    }
    for text in [
        &value.content_revision,
        &value.ruleset_revision,
        &value.sim_revision,
    ] {
        mint::check_content_key(text)?;
    }
    if value.session_generation == 0 || value.lease_generation == 0 {
        return Err(AuditError::InvalidInput);
    }
    let before = value.before.as_ref().ok_or(AuditError::InvalidInput)?;
    let after = value.after.as_ref().ok_or(AuditError::InvalidInput)?;
    mint::check_uuid_v7(&before.world_id)?;
    mint::check_definition(before.definition.as_ref())?;
    let slot = u8::try_from(value.source_slot).map_err(|_| AuditError::InvalidInput)?;
    let (quantity, lifecycle) =
        plan(burn, slot, before.quantity, 0).map_err(|_| AuditError::InvalidInput)?;
    if before.item_instance_id != value.item_instance
        || before.lifecycle != 1
        || before.key_number > 65535
        || after
            != &(OneItemStateV1 {
                quantity,
                lifecycle,
                ..before.clone()
            })
        || (slot == 0) != value.source_entry.is_some()
    {
        return Err(AuditError::InvalidInput);
    }
    if let Some(entry) = &value.source_entry {
        mint::check_uuid_v7(&entry.parent_item_instance_id)?;
        if entry.placement_ordinal == 0 || entry.parent_item_instance_id == value.item_instance {
            return Err(AuditError::InvalidInput);
        }
    }
    if burn {
        if value.ground.is_some() || value.typed_sink != AMULET_SINK {
            return Err(AuditError::InvalidInput);
        }
    } else {
        let ground = value.ground.as_ref().ok_or(AuditError::InvalidInput)?;
        mint::check_uuid_v7(&ground.channel_id)?;
        mint::check_content_key(&ground.map_revision)?;
        mint::check_technical_bytes(&ground.spatial_position)?;
        if ground.world_id != before.world_id
            || ground.content_revision != value.content_revision
            || ground.corpse_ref != value.death_occurrence
            || ground.native_room_placement_context != value.death_occurrence
            || ground.runtime_scope_ownership_generation == 0
            || !value.typed_sink.is_empty()
        {
            return Err(AuditError::InvalidInput);
        }
    }
    Ok(())
}
pub fn decode(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, bool, OneItemPlayerDeathItemV1), AuditError> {
    let envelope = mint::decode_common_envelope_within(wire, mint::RL07_ENVELOPE_BYTES_MAX)?;
    if envelope.payload.len() > mint::RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let payload =
        Payload::decode(envelope.payload.as_slice()).map_err(|_| AuditError::InvalidInput)?;
    if payload.interpretation_revision != 1 || payload.encode_to_vec() != envelope.payload {
        return Err(AuditError::InvalidInput);
    }
    let (burn, value) = match payload.operation {
        Some(Operation::Drop(v)) => (false, v),
        Some(Operation::Amulet(v)) => (true, v),
        None => return Err(AuditError::InvalidInput),
    };
    check(burn, &value)?;
    if envelope.command_id.is_some()
        || envelope.game_session_id.is_some()
        || envelope.connection_generation.is_some()
        || envelope.causation.is_some()
        || envelope.world_id.as_ref() != value.before.as_ref().map(|s| &s.world_id)
        || envelope
            .transaction_event
            .as_ref()
            .is_none_or(|t| t.ordinal != 1 || t.count != 1)
        || (burn && envelope.channel_id.is_some())
        || (!burn && envelope.channel_id.as_ref() != value.ground.as_ref().map(|g| &g.channel_id))
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((envelope, burn, value))
}
pub(super) fn encode(
    request: &PlayerDeathItemRequest,
    burn: bool,
    value: OneItemPlayerDeathItemV1,
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, AuditError> {
    check(burn, &value)?;
    mint::check_uuid_v7(&request.event_id)?;
    mint::check_uuid_v7(&request.transaction_id)?;
    mint::check_technical_text(&request.server_build_id)?;
    if request.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    let world = value.before.as_ref().map(|s| s.world_id.clone());
    let channel = value.ground.as_ref().map(|g| g.channel_id.clone());
    let payload = mint::encode_bounded(
        &Payload {
            interpretation_revision: 1,
            operation: Some(if burn {
                Operation::Amulet(value)
            } else {
                Operation::Drop(value)
            }),
        },
        mint::RL07_PAYLOAD_BYTES_MAX,
    )?;
    let envelope = EventEnvelopeV1 {
        envelope_revision: 1,
        event_id: request.event_id.to_vec(),
        event_type_id: 2,
        event_schema_revision: tuple.schema_revision(),
        durability_class: mint::DURABLE_AUDIT,
        privacy_class: mint::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
        occurred_at_unix_ms: request.occurred_at_unix_ms,
        world_id: world,
        channel_id: channel,
        transaction_event: Some(mint::TransactionEventRefV1 {
            transaction_id: request.transaction_id.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        server_build_id: request.server_build_id.clone(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..EventEnvelopeV1::default()
    };
    let wire = mint::encode_bounded(&envelope, mint::RL07_ENVELOPE_BYTES_MAX)?;
    check_usage(
        burn,
        [
            1,
            if burn { 1 } else { 2 },
            0,
            0,
            0,
            2,
            3,
            1,
            envelope.payload.len() as u64,
            wire.len() as u64,
            1,
        ],
    )?;
    decode(&wire)?;
    Ok(wire)
}
