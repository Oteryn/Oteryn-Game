//! HOUSE-RUNTIME-1a immutable house-side audit validation.
//!
//! Native type-2 House transfer codec for the three accepted shapes. The additive
//! operation tag 11 is this CP-slot candidate, subject to numeric/architect review.
//! Event validation checks immutable facts, never independently current authority.

use super::house_interior::{HouseInteriorEffect, HouseInteriorError};
use crate::domain::ItemInstanceId;
use crate::foundation::CharacterId;

/// House-side facts to be carried by the registered one-item operation, together
/// with the character-side location, item state, command and normal envelope facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseInteriorAuditChange {
    pub item_instance_id: ItemInstanceId,
    pub actor: CharacterId,
    pub effect: HouseInteriorEffect,
}

impl HouseInteriorAuditChange {
    pub fn validate(&self) -> Result<(), HouseInteriorError> {
        self.effect.validate(self.item_instance_id, self.actor)
    }
}

use super::item_mint_audit::{
    self as audit, AuditError, CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1,
    OneItemOperationV1, OneItemStateV1, OneItemTransactionV1, TransactionEventRefV1,
    Type2EventTuple,
};
use super::item_transfer_audit::{OneItemTransferCauseV1, TransferEventIdentity};
use prost::Message;
use sha2::{Digest, Sha256};

/// House custody, including the immutable reclaim binding. It carries no ChannelId.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemHouseLocationV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub world_id: Vec<u8>,
    #[prost(string, tag = "2")]
    pub house_key: String,
    #[prost(bytes = "vec", tag = "3")]
    pub spatial_position: Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub stack_ordinal: u64,
    #[prost(uint64, tag = "5")]
    pub scope_ownership_generation: u64,
    #[prost(bytes = "vec", tag = "6")]
    pub placement_transaction_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "7")]
    pub reclaim_subject: Vec<u8>,
    #[prost(uint64, tag = "8")]
    pub provenance_revision: u64,
}

/// Exactly one actual native Character custody location. Equipment slot 9 is the B3
/// container slot; the other slots are the native equipment owner's 1..=10 slots.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemHouseCharacterLocationV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub parent_item_instance_id: Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub placement_ordinal: u64,
    #[prost(uint32, tag = "4")]
    pub equipment_slot: u32,
}

#[derive(Clone, PartialEq, Eq, prost::Oneof)]
pub enum HouseTransferSourceV1 {
    #[prost(message, tag = "3")]
    House(OneItemHouseLocationV1),
    #[prost(message, tag = "4")]
    Character(OneItemHouseCharacterLocationV1),
}
#[derive(Clone, PartialEq, Eq, prost::Oneof)]
pub enum HouseTransferDestinationV1 {
    #[prost(message, tag = "5")]
    House(OneItemHouseLocationV1),
    #[prost(message, tag = "6")]
    Inventory(OneItemHouseCharacterLocationV1),
}

/// The additive candidate operation. Numeric tag 11 remains subject to the slot's
/// CP/architect review; it is not a client protocol command or capability.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemHouseTransferV1 {
    #[prost(message, optional, tag = "1")]
    pub before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub after: Option<OneItemStateV1>,
    #[prost(oneof = "HouseTransferSourceV1", tags = "3, 4")]
    pub source: Option<HouseTransferSourceV1>,
    #[prost(oneof = "HouseTransferDestinationV1", tags = "5, 6")]
    pub destination: Option<HouseTransferDestinationV1>,
    #[prost(message, optional, tag = "7")]
    pub cause: Option<OneItemTransferCauseV1>,
    #[prost(bytes = "vec", tag = "8")]
    pub character_id: Vec<u8>,
    #[prost(uint64, tag = "9")]
    pub connection_generation: u64,
    #[prost(uint64, tag = "10")]
    pub character_lease_generation: u64,
}

fn check_house(location: &OneItemHouseLocationV1, world: &[u8]) -> Result<(), AuditError> {
    audit::check_uuid_v7(&location.world_id)?;
    audit::check_uuid_v7(&location.placement_transaction_id)?;
    audit::check_uuid_v7(&location.reclaim_subject)?;
    audit::check_technical_bytes(&location.spatial_position)?;
    let world_id = crate::foundation::WorldId::decode(&location.world_id)
        .map_err(|_| AuditError::InvalidInput)?;
    if location.world_id != world
        || super::house_scope_handoff::HouseId::new(world_id, &location.house_key).is_none()
        || location.stack_ordinal == 0
        || location.scope_ownership_generation == 0
        || location.provenance_revision == 0
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}
fn check_character(
    location: &OneItemHouseCharacterLocationV1,
    actor: &[u8],
    inventory: bool,
) -> Result<(), AuditError> {
    audit::check_uuid_v7(&location.character_id)?;
    if location.character_id != actor {
        return Err(AuditError::InvalidInput);
    }
    if location.equipment_slot == 0 {
        audit::check_uuid_v7(&location.parent_item_instance_id)?;
        if location.placement_ordinal == 0 {
            return Err(AuditError::InvalidInput);
        }
    } else if inventory
        || location.equipment_slot > 10
        || !location.parent_item_instance_id.is_empty()
        || location.placement_ordinal != 0
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

/// Closed three-shape grammar. This proves event relationships, never current authority.
pub fn check_house_transfer(value: &OneItemHouseTransferV1) -> Result<(), AuditError> {
    let before = value.before.as_ref().ok_or(AuditError::InvalidInput)?;
    let after = value.after.as_ref().ok_or(AuditError::InvalidInput)?;
    audit::check_uuid_v7(&before.item_instance_id)?;
    audit::check_uuid_v7(&before.world_id)?;
    audit::check_uuid_v7(&value.character_id)?;
    audit::check_definition(before.definition.as_ref())?;
    audit::check_key_number(before)?;
    if before != after
        || before.lifecycle != audit::ITEM_LIFECYCLE_LIVE
        || !(1..=100).contains(&before.quantity)
        || value.connection_generation == 0
        || value.character_lease_generation == 0
    {
        return Err(AuditError::InvalidInput);
    }
    let cause = value.cause.as_ref().ok_or(AuditError::InvalidInput)?;
    let command = cause.command_ref.as_ref().ok_or(AuditError::InvalidInput)?;
    audit::check_uuid_v7(&command.game_session_id)?;
    for text in [
        &cause.content_revision,
        &cause.ruleset_revision,
        &cause.sim_revision,
    ] {
        audit::check_content_key(text)?;
    }
    if cause.typed_cause != "house_interior_transfer" || command.command_id == 0 {
        return Err(AuditError::InvalidInput);
    }
    match (value.source.as_ref(), value.destination.as_ref()) {
        (
            Some(HouseTransferSourceV1::Character(source)),
            Some(HouseTransferDestinationV1::House(destination)),
        ) => {
            check_character(source, &value.character_id, false)?;
            check_house(destination, &before.world_id)?;
            if destination.reclaim_subject != value.character_id
                || destination.provenance_revision != 1
            {
                return Err(AuditError::InvalidInput);
            }
        }
        (
            Some(HouseTransferSourceV1::House(source)),
            Some(HouseTransferDestinationV1::Inventory(destination)),
        ) => {
            check_house(source, &before.world_id)?;
            check_character(destination, &value.character_id, true)?;
        }
        (
            Some(HouseTransferSourceV1::House(source)),
            Some(HouseTransferDestinationV1::House(destination)),
        ) => {
            check_house(source, &before.world_id)?;
            check_house(destination, &before.world_id)?;
            if source.house_key != destination.house_key
                || source.scope_ownership_generation != destination.scope_ownership_generation
                || source.placement_transaction_id == destination.placement_transaction_id
                || source.reclaim_subject != destination.reclaim_subject
                || source.provenance_revision.checked_add(1)
                    != Some(destination.provenance_revision)
            {
                return Err(AuditError::InvalidInput);
            }
        }
        _ => return Err(AuditError::InvalidInput),
    }
    Ok(())
}

pub fn decode_house_transfer_payload(bytes: &[u8]) -> Result<OneItemHouseTransferV1, AuditError> {
    if bytes.len() > audit::RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let transaction = OneItemTransactionV1::decode(bytes).map_err(|_| AuditError::InvalidInput)?;
    if transaction.encode_to_vec() != bytes
        || transaction.interpretation_revision != audit::INTERPRETATION_REVISION
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::HouseTransfer(value)) = transaction.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_house_transfer(&value)?;
    Ok(value)
}

pub fn decode_house_transfer_envelope(
    bytes: &[u8],
) -> Result<(EventEnvelopeV1, OneItemHouseTransferV1), AuditError> {
    let envelope = audit::decode_common_envelope(bytes)?;
    let value = decode_house_transfer_payload(&envelope.payload)?;
    let state = value.before.as_ref().ok_or(AuditError::InvalidInput)?;
    let command = value
        .cause
        .as_ref()
        .and_then(|c| c.command_ref.as_ref())
        .ok_or(AuditError::InvalidInput)?;
    if envelope.world_id.as_deref() != Some(state.world_id.as_slice())
        || envelope.channel_id.is_some()
        || envelope.game_session_id.as_deref() != Some(command.game_session_id.as_slice())
        || envelope.connection_generation != Some(value.connection_generation)
        || envelope.command_id != Some(command.command_id)
        || envelope.causation
            != Some(CausationRefV1 {
                cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                    game_session_id: command.game_session_id.clone(),
                    command_id: command.command_id,
                })),
            })
    {
        return Err(AuditError::InvalidInput);
    }
    if let Some(HouseTransferDestinationV1::House(destination)) = &value.destination
        && envelope
            .transaction_event
            .as_ref()
            .map(|t| t.transaction_id.as_slice())
            != Some(destination.placement_transaction_id.as_slice())
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((envelope, value))
}

pub fn encode_house_transfer_event(
    identity: TransferEventIdentity<'_>,
    value: OneItemHouseTransferV1,
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, AuditError> {
    check_house_transfer(&value)?;
    audit::check_uuid_v7(&identity.event_id)?;
    audit::check_uuid_v7(&identity.transaction_id)?;
    audit::check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    let world = value
        .before
        .as_ref()
        .ok_or(AuditError::InvalidInput)?
        .world_id
        .clone();
    let command = value
        .cause
        .as_ref()
        .and_then(|c| c.command_ref.clone())
        .ok_or(AuditError::InvalidInput)?;
    let generation = value.connection_generation;
    let payload = audit::encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: audit::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::HouseTransfer(value)),
        },
        audit::RL07_PAYLOAD_BYTES_MAX,
    )?;
    let envelope = EventEnvelopeV1 {
        envelope_revision: audit::ENVELOPE_REVISION,
        event_id: identity.event_id.to_vec(),
        event_type_id: audit::EVENT_TYPE_ID,
        event_schema_revision: tuple.schema_revision(),
        durability_class: audit::DURABLE_AUDIT,
        privacy_class: audit::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
        occurred_at_unix_ms: identity.occurred_at_unix_ms,
        world_id: Some(world),
        game_session_id: Some(command.game_session_id.clone()),
        connection_generation: Some(generation),
        command_id: Some(command.command_id),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: identity.transaction_id.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(CausationRefV1 {
            cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
                game_session_id: command.game_session_id,
                command_id: command.command_id,
            })),
        }),
        server_build_id: identity.server_build_id.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..EventEnvelopeV1::default()
    };
    let bytes = audit::encode_bounded(&envelope, audit::RL07_ENVELOPE_BYTES_MAX)?;
    decode_house_transfer_envelope(&bytes)?;
    Ok(bytes)
}
