//! Registered DUR-03 TRANSFER durable-audit encoding: event type 2
//! (`oteryn.events.v1.OneItemTransactionV1`, operation `transfer`, oneof tag
//! 3) in the normative ANL-01 `EventEnvelope`, for the B3 destinations
//! (decision `B3-INVENTORY-DESTINATION-CAPACITY-STACKS-V1` §4): the
//! CharacterEquipment `container` slot, a direct entry of the equipped main
//! backpack, and the D83 full-merge and top-up shapes.
//!
//! One live Ground item A moves in every shape. The optional receiver B is a
//! compatible stack already in the main backpack; it shares A's World and
//! definition, so only its identity, position and two quantities are carried.
//! Stack quantities are item state (before/after), never value lines (RL-03
//! stays 0). Every registered bound is checked before encode allocation and
//! oversize input is rejected, never truncated.

use super::item_mint_audit::{
    self as mint, AuditError, CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1,
    ITEM_LIFECYCLE_LIVE, OneItemGroundV1, OneItemOperationV1, OneItemStateV1, OneItemTransactionV1,
    RL07_ENVELOPE_BYTES_MAX, RL07_PAYLOAD_BYTES_MAX, TransactionEventRefV1,
    TransactionResourceUsage, check_content_key, check_definition, check_technical_bytes,
    check_uuid_v7, encode_bounded,
};
use prost::Message;
use sha2::{Digest, Sha256};

/// Typed cause of a player-originated Ground pickup TRANSFER.
pub const GROUND_PICKUP_TYPED_CAUSE: &str = "ground_pickup_transfer";
/// `OneItemStateV1.lifecycle` of a source retired by a full merge (DUR-03 §11.5).
pub const ITEM_LIFECYCLE_RETIRED: u32 = 2;
/// GAMEITEM01-STACK-QUANTITY-MAX: absolute stack ceiling (D82).
pub const GAMEITEM01_STACK_QUANTITY_MAX: u32 = 100;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemContainerEntryV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub parent_item_instance_id: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub placement_ordinal: u64,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemCommandRefV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub game_session_id: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub command_id: u64,
}

/// Character-owned destination. The direct-root inventory `typed_position`
/// (field 2) stays reserved; exactly one B3 destination is set, or none for a
/// full merge (the source retires into the receiver).
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemInventoryV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub character_id: Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub expected_session_generation: u64,
    #[prost(bytes = "vec", tag = "4")]
    pub expected_game_session_id: Vec<u8>,
    #[prost(uint64, tag = "5")]
    pub expected_character_lease_generation: u64,
    #[prost(message, optional, tag = "6")]
    pub container_entry: Option<OneItemContainerEntryV1>,
    #[prost(bool, tag = "7")]
    pub equipment_container_slot: bool,
}

/// TRANSFER cause: the actual player CommandRef and the interpretation
/// revisions. Same field numbers as `OneItemProvenanceV1`; the MINT-only
/// death/loot fields (2-5) are absent.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemTransferCauseV1 {
    #[prost(string, tag = "1")]
    pub typed_cause: String,
    #[prost(string, tag = "6")]
    pub content_revision: String,
    #[prost(string, tag = "7")]
    pub ruleset_revision: String,
    #[prost(string, tag = "8")]
    pub sim_revision: String,
    #[prost(message, optional, tag = "9")]
    pub command_ref: Option<OneItemCommandRefV1>,
}

/// D83 receiver B: a compatible stack in the main backpack.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemReceiverV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub item_instance_id: Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub position: Option<OneItemContainerEntryV1>,
    #[prost(uint32, tag = "3")]
    pub quantity_before: u32,
    #[prost(uint32, tag = "4")]
    pub quantity_after: u32,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemTransferV1 {
    #[prost(message, optional, tag = "1")]
    pub before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub after: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "3")]
    pub source: Option<OneItemGroundV1>,
    #[prost(message, optional, tag = "4")]
    pub destination: Option<OneItemInventoryV1>,
    #[prost(message, optional, tag = "5")]
    pub cause: Option<OneItemTransferCauseV1>,
    #[prost(message, optional, tag = "6")]
    pub receiver: Option<OneItemReceiverV1>,
}

/// Closed TRANSFER shapes (B3 §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferShape {
    /// An empty container moves into the empty CharacterEquipment `container` slot.
    ContainerSlot,
    /// A new entry (new placement ordinal) of the main backpack.
    NewEntry,
    /// B grows by A's quantity; A retires.
    FullMerge,
    /// B grows to the stack maximum; A keeps the remainder in a new entry.
    TopUp,
}

impl TransferShape {
    /// Registered DUR-03 usage of each shape (B3 §4.5).
    #[must_use]
    pub const fn usage(self) -> TransactionResourceUsage {
        let (touched, lines, participants, work) = match self {
            // A: participant + Ground removal + placement.
            Self::ContainerSlot | Self::NewEntry => (1, 2, 1, 3),
            // A: participant + Ground removal + retirement; B: participant + quantity.
            Self::FullMerge => (2, 1, 2, 5),
            // A: participant + Ground removal + placement + quantity; B: 2.
            Self::TopUp => (2, 2, 2, 6),
        };
        TransactionResourceUsage {
            touched_item_instances: touched,
            location_custody_lines: lines,
            value_lines: 0,
            transform_lines: 0,
            container_expansion: 0,
            participants,
            effect_work_units: work,
            events: 1,
        }
    }
}

fn check_entry(value: &OneItemContainerEntryV1) -> Result<(), AuditError> {
    check_uuid_v7(&value.parent_item_instance_id)?;
    if value.placement_ordinal == 0 {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

fn check_state(value: &OneItemStateV1) -> Result<(), AuditError> {
    check_uuid_v7(&value.item_instance_id)?;
    check_uuid_v7(&value.world_id)?;
    check_definition(value.definition.as_ref())
}

/// Complete closed TRANSFER shape, conservation and every registered bound.
pub fn check_transfer(value: &OneItemTransferV1) -> Result<TransferShape, AuditError> {
    let before = value.before.as_ref().ok_or(AuditError::InvalidInput)?;
    let after = value.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let ground = value.source.as_ref().ok_or(AuditError::InvalidInput)?;
    let destination = value.destination.as_ref().ok_or(AuditError::InvalidInput)?;
    let cause = value.cause.as_ref().ok_or(AuditError::InvalidInput)?;
    let command = cause.command_ref.as_ref().ok_or(AuditError::InvalidInput)?;
    check_state(before)?;
    check_state(after)?;
    check_uuid_v7(&ground.world_id)?;
    check_uuid_v7(&ground.channel_id)?;
    check_technical_bytes(&ground.spatial_position)?;
    check_technical_bytes(&ground.corpse_ref)?;
    check_content_key(&ground.map_revision)?;
    check_content_key(&ground.content_revision)?;
    check_technical_bytes(&ground.native_room_placement_context)?;
    check_uuid_v7(&destination.character_id)?;
    check_uuid_v7(&destination.expected_game_session_id)?;
    if let Some(entry) = destination.container_entry.as_ref() {
        check_entry(entry)?;
    }
    check_content_key(&cause.content_revision)?;
    check_content_key(&cause.ruleset_revision)?;
    check_content_key(&cause.sim_revision)?;
    check_uuid_v7(&command.game_session_id)?;
    // The same live item leaves the actual Ground of its own World.
    if before.item_instance_id != after.item_instance_id
        || before.world_id != after.world_id
        || before.definition != after.definition
        || before.lifecycle != ITEM_LIFECYCLE_LIVE
        || before.quantity == 0
        || before.quantity > GAMEITEM01_STACK_QUANTITY_MAX
        || ground.world_id != before.world_id
        || ground.runtime_scope_ownership_generation == 0
        || cause.typed_cause != GROUND_PICKUP_TYPED_CAUSE
        || command.command_id == 0
        || command.game_session_id != destination.expected_game_session_id
        || destination.expected_session_generation == 0
    {
        return Err(AuditError::InvalidInput);
    }
    let shape = match (
        destination.equipment_container_slot,
        destination.container_entry.as_ref(),
        value.receiver.as_ref(),
    ) {
        (true, None, None) => TransferShape::ContainerSlot,
        (false, Some(_), None) => TransferShape::NewEntry,
        (false, None, Some(_)) => TransferShape::FullMerge,
        (false, Some(_), Some(_)) => TransferShape::TopUp,
        _ => return Err(AuditError::InvalidInput),
    };
    let moved = match shape {
        TransferShape::ContainerSlot | TransferShape::NewEntry => {
            if after.lifecycle != ITEM_LIFECYCLE_LIVE || after.quantity != before.quantity {
                return Err(AuditError::InvalidInput);
            }
            0
        }
        TransferShape::FullMerge => {
            if after.lifecycle != ITEM_LIFECYCLE_RETIRED || after.quantity != 0 {
                return Err(AuditError::InvalidInput);
            }
            before.quantity
        }
        TransferShape::TopUp => {
            if after.lifecycle != ITEM_LIFECYCLE_LIVE
                || after.quantity == 0
                || after.quantity >= before.quantity
            {
                return Err(AuditError::InvalidInput);
            }
            before.quantity - after.quantity
        }
    };
    if let Some(receiver) = value.receiver.as_ref() {
        check_uuid_v7(&receiver.item_instance_id)?;
        let position = receiver.position.as_ref().ok_or(AuditError::InvalidInput)?;
        check_entry(position)?;
        // Exact units are conserved (SPLIT_MERGE_QUANTITY) within the ceiling.
        if receiver.item_instance_id == before.item_instance_id
            || receiver.quantity_before == 0
            || receiver.quantity_after > GAMEITEM01_STACK_QUANTITY_MAX
            || receiver
                .quantity_after
                .checked_sub(receiver.quantity_before)
                != Some(moved)
            || destination.container_entry.as_ref().is_some_and(|entry| {
                entry.parent_item_instance_id != position.parent_item_instance_id
                    || entry.placement_ordinal <= position.placement_ordinal
            })
        {
            return Err(AuditError::InvalidInput);
        }
    }
    shape.usage().check()?;
    Ok(shape)
}

/// RL-07 payload gate for TRANSFER: size before decode, canonical round trip,
/// closed shape and per-field bounds.
pub fn decode_transfer_payload(wire: &[u8]) -> Result<OneItemTransferV1, AuditError> {
    if wire.len() > RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = OneItemTransactionV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire
        || value.interpretation_revision != mint::INTERPRETATION_REVISION
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::Transfer(transfer)) = value.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_transfer(&transfer)?;
    Ok(transfer)
}

/// RL-07 envelope gate for TRANSFER: the shared envelope gate, the payload
/// gate, and the envelope's scope and player command equal to the payload's.
pub fn decode_transfer_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, OneItemTransferV1), AuditError> {
    let value = mint::decode_common_envelope(wire)?;
    let transfer = decode_transfer_payload(&value.payload)?;
    let ground = transfer.source.as_ref().ok_or(AuditError::InvalidInput)?;
    let command = transfer
        .cause
        .as_ref()
        .and_then(|cause| cause.command_ref.as_ref())
        .ok_or(AuditError::InvalidInput)?;
    let destination = transfer
        .destination
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    let envelope_command = CausationRefV1 {
        cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
            game_session_id: command.game_session_id.clone(),
            command_id: command.command_id,
        })),
    };
    if value.world_id.as_deref() != Some(ground.world_id.as_slice())
        || value.channel_id.as_deref() != Some(ground.channel_id.as_slice())
        || value.game_session_id.as_deref() != Some(command.game_session_id.as_slice())
        || value.command_id != Some(command.command_id)
        || value.connection_generation != Some(destination.expected_session_generation)
        || value.causation.as_ref() != Some(&envelope_command)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((value, transfer))
}

/// Envelope identity fixed before the commit attempt that can become ambiguous.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: &'a str,
}

/// Build the exact immutable event bytes of one TRANSFER; re-decoded through
/// both gates before they are returned.
pub fn encode_transfer_event(
    identity: TransferEventIdentity<'_>,
    transfer: OneItemTransferV1,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    mint::check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    check_transfer(&transfer)?;
    let ground = transfer.source.as_ref().ok_or(AuditError::InvalidInput)?;
    let (world_id, channel_id) = (ground.world_id.clone(), ground.channel_id.clone());
    let command = transfer
        .cause
        .as_ref()
        .and_then(|cause| cause.command_ref.clone())
        .ok_or(AuditError::InvalidInput)?;
    let connection_generation = transfer
        .destination
        .as_ref()
        .map(|destination| destination.expected_session_generation)
        .ok_or(AuditError::InvalidInput)?;
    let payload = encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::Transfer(transfer)),
        },
        RL07_PAYLOAD_BYTES_MAX,
    )?;
    let envelope = EventEnvelopeV1 {
        envelope_revision: mint::ENVELOPE_REVISION,
        event_id: identity.event_id.to_vec(),
        event_type_id: mint::EVENT_TYPE_ID,
        event_schema_revision: mint::EVENT_SCHEMA_REVISION,
        durability_class: mint::DURABLE_AUDIT,
        privacy_class: mint::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: mint::RETENTION_PROFILE_ID.into(),
        occurred_at_unix_ms: identity.occurred_at_unix_ms,
        world_id: Some(world_id),
        channel_id: Some(channel_id),
        game_session_id: Some(command.game_session_id.clone()),
        connection_generation: Some(connection_generation),
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
    let wire = encode_bounded(&envelope, RL07_ENVELOPE_BYTES_MAX)?;
    let (decoded, _) = decode_transfer_envelope(&wire)?;
    if decoded != envelope {
        return Err(AuditError::InvalidInput);
    }
    Ok(wire)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::super::item_mint_audit::{
        OneItemTypedDefinitionRevisionV1, RL01_TOUCHED_ITEM_INSTANCES_MAX,
        RL02_LOCATION_CUSTODY_LINES_MAX, RL03_VALUE_LINES_MAX, RL06_EFFECT_WORK_UNITS_MAX,
        RL06_PARTICIPANTS_MAX, RL07_CONTENT_KEY_BYTES_MAX, RL07_TECHNICAL_FIELD_BYTES_MAX,
        decode_envelope, decode_payload, tests::worst_case_envelope,
    };
    use super::*;

    fn uuid(tag: u8) -> Vec<u8> {
        let mut bytes = vec![0; 16];
        bytes[5] = 1;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = tag;
        bytes
    }

    fn state(quantity: u32, lifecycle: u32) -> OneItemStateV1 {
        OneItemStateV1 {
            item_instance_id: uuid(9),
            world_id: uuid(1),
            definition: Some(OneItemTypedDefinitionRevisionV1 {
                family: "Item".into(),
                production_key: "fixture:b3.coin".into(),
                revision_ref: "definition-r1".into(),
            }),
            quantity,
            lifecycle,
        }
    }

    fn entry(ordinal: u64) -> OneItemContainerEntryV1 {
        OneItemContainerEntryV1 {
            parent_item_instance_id: uuid(20),
            placement_ordinal: ordinal,
        }
    }

    fn transfer(shape: TransferShape) -> OneItemTransferV1 {
        let (after, entry_at, receiver, slot) = match shape {
            TransferShape::ContainerSlot => (state(1, 1), None, None, true),
            TransferShape::NewEntry => (state(30, 1), Some(entry(3)), None, false),
            TransferShape::FullMerge => (
                state(0, ITEM_LIFECYCLE_RETIRED),
                None,
                Some((60, 90)),
                false,
            ),
            TransferShape::TopUp => (state(20, 1), Some(entry(3)), Some((90, 100)), false),
        };
        OneItemTransferV1 {
            before: Some(state(30, 1)),
            after: Some(if shape == TransferShape::ContainerSlot {
                OneItemStateV1 {
                    quantity: 30,
                    ..after
                }
            } else {
                after
            }),
            source: Some(OneItemGroundV1 {
                world_id: uuid(1),
                channel_id: uuid(2),
                spatial_position: vec![1, 2, 3],
                corpse_ref: uuid(3),
                map_revision: "map-1".into(),
                content_revision: "content-1".into(),
                native_room_placement_context: uuid(6),
                runtime_scope_ownership_generation: 1,
            }),
            destination: Some(OneItemInventoryV1 {
                character_id: uuid(41),
                expected_session_generation: 1,
                expected_game_session_id: uuid(50),
                expected_character_lease_generation: 1,
                container_entry: entry_at,
                equipment_container_slot: slot,
            }),
            cause: Some(OneItemTransferCauseV1 {
                typed_cause: GROUND_PICKUP_TYPED_CAUSE.into(),
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
                command_ref: Some(OneItemCommandRefV1 {
                    game_session_id: uuid(50),
                    command_id: 7,
                }),
            }),
            receiver: receiver.map(|(before, after)| OneItemReceiverV1 {
                item_instance_id: uuid(10),
                position: Some(entry(2)),
                quantity_before: before,
                quantity_after: after,
            }),
        }
    }

    fn identity() -> TransferEventIdentity<'static> {
        let mut event_id = [0; 16];
        event_id.copy_from_slice(&uuid(30));
        let mut transaction_id = [0; 16];
        transaction_id.copy_from_slice(&uuid(40));
        TransferEventIdentity {
            event_id,
            transaction_id,
            occurred_at_unix_ms: 1_790_000_000_000,
            server_build_id: "oteryn-game-server/test",
        }
    }

    const SHAPES: [TransferShape; 4] = [
        TransferShape::ContainerSlot,
        TransferShape::NewEntry,
        TransferShape::FullMerge,
        TransferShape::TopUp,
    ];

    #[test]
    fn every_shape_round_trips_and_mint_gates_reject_it() {
        for shape in SHAPES {
            let wire = encode_transfer_event(identity(), transfer(shape)).unwrap();
            let (envelope, decoded) = decode_transfer_envelope(&wire).unwrap();
            assert_eq!(decoded, transfer(shape), "{shape:?}");
            assert_eq!(check_transfer(&decoded), Ok(shape));
            assert_eq!(envelope.command_id, Some(7));
            assert_eq!(envelope.game_session_id, Some(uuid(50)));
            assert_eq!(
                decode_envelope(&wire).map(|_| ()),
                Err(AuditError::InvalidInput)
            );
            assert_eq!(
                decode_payload(&envelope.payload).map(|_| ()),
                Err(AuditError::InvalidInput)
            );
        }
    }

    #[test]
    fn registered_shape_usage_is_max_and_max_plus_one() {
        // B3 §4.5: touched 2, participants 2, work units 6, value lines 0.
        assert_eq!(RL01_TOUCHED_ITEM_INSTANCES_MAX, 2);
        assert_eq!(RL06_PARTICIPANTS_MAX, 2);
        assert_eq!(RL06_EFFECT_WORK_UNITS_MAX, 6);
        assert_eq!(RL02_LOCATION_CUSTODY_LINES_MAX, 2);
        assert_eq!(RL03_VALUE_LINES_MAX, 0);
        let top_up = TransferShape::TopUp.usage();
        assert_eq!(
            (
                top_up.touched_item_instances,
                top_up.participants,
                top_up.effect_work_units,
                top_up.location_custody_lines,
            ),
            (2, 2, 6, 2)
        );
        let full = TransferShape::FullMerge.usage();
        assert_eq!(
            (full.touched_item_instances, full.effect_work_units),
            (2, 5)
        );
        for shape in SHAPES {
            assert!(shape.usage().check().is_ok(), "{shape:?}");
        }
        let over: [fn(&mut TransactionResourceUsage); 4] = [
            |u| u.touched_item_instances += 1,
            |u| u.participants += 1,
            |u| u.effect_work_units += 1,
            |u| u.location_custody_lines += 1,
        ];
        for bump in over {
            let mut usage = top_up;
            bump(&mut usage);
            assert_eq!(usage.check(), Err(AuditError::CapacityExceeded));
        }
        let mut value_line = top_up;
        value_line.value_lines = 1;
        assert_eq!(value_line.check(), Err(AuditError::InvalidInput));
    }

    #[test]
    fn conservation_shape_and_ceiling_violations_reject() {
        let bad: [fn(&mut OneItemTransferV1); 10] = [
            // A unit appears from nowhere.
            |t| t.receiver.as_mut().unwrap().quantity_after += 1,
            // Receiver above the stack ceiling (101).
            |t| {
                let receiver = t.receiver.as_mut().unwrap();
                receiver.quantity_before = 71;
                receiver.quantity_after = 101;
            },
            // A different item arrives.
            |t| t.after.as_mut().unwrap().item_instance_id = uuid(11),
            // Receiver is the source itself.
            |t| t.receiver.as_mut().unwrap().item_instance_id = uuid(9),
            // Two destinations at once.
            |t| t.destination.as_mut().unwrap().equipment_container_slot = true,
            // A foreign session's command.
            |t| {
                t.cause
                    .as_mut()
                    .unwrap()
                    .command_ref
                    .as_mut()
                    .unwrap()
                    .game_session_id = uuid(51)
            },
            // Top-up remainder placed at an older ordinal than the receiver.
            |t| t.destination.as_mut().unwrap().container_entry = Some(entry(1)),
            // A MINT cause on a TRANSFER.
            |t| t.cause.as_mut().unwrap().typed_cause = "loot_mint".into(),
            // Source quantity above the ceiling.
            |t| t.before.as_mut().unwrap().quantity = 101,
            // Missing receiver position.
            |t| t.receiver.as_mut().unwrap().position = None,
        ];
        for (index, mutate) in bad.into_iter().enumerate() {
            let mut value = transfer(TransferShape::TopUp);
            mutate(&mut value);
            assert!(check_transfer(&value).is_err(), "case {index}");
            assert!(
                encode_transfer_event(identity(), value).is_err(),
                "case {index}"
            );
        }
        let mut retired_slot = transfer(TransferShape::ContainerSlot);
        retired_slot.after.as_mut().unwrap().lifecycle = ITEM_LIFECYCLE_RETIRED;
        assert_eq!(check_transfer(&retired_slot), Err(AuditError::InvalidInput));
        // Stack ceiling max: receiver reaching exactly 100 is accepted.
        assert!(check_transfer(&transfer(TransferShape::TopUp)).is_ok());
    }

    #[test]
    fn envelope_scope_and_command_must_equal_the_payload() {
        let wire = encode_transfer_event(identity(), transfer(TransferShape::NewEntry)).unwrap();
        let tamper: [fn(&mut EventEnvelopeV1); 4] = [
            |e| e.command_id = Some(8),
            |e| e.game_session_id = None,
            |e| e.causation = None,
            |e| e.channel_id = Some(uuid(8)),
        ];
        for mutate in tamper {
            let mut envelope = EventEnvelopeV1::decode(wire.as_slice()).unwrap();
            mutate(&mut envelope);
            assert_eq!(
                decode_transfer_envelope(&envelope.encode_to_vec()).map(|_| ()),
                Err(AuditError::InvalidInput)
            );
        }
    }

    // Byte-bound probe of the merge shapes (B3 §4.5 RL-07): every bounded
    // field at its maximum and every varint at its widest. Not admissible.
    fn worst_case_top_up() -> (usize, usize) {
        let content = "k".repeat(RL07_CONTENT_KEY_BYTES_MAX);
        let technical = "t".repeat(RL07_TECHNICAL_FIELD_BYTES_MAX);
        let technical_bytes = vec![0xa5; RL07_TECHNICAL_FIELD_BYTES_MAX];
        let typed = OneItemTypedDefinitionRevisionV1 {
            family: technical.clone(),
            production_key: content.clone(),
            revision_ref: content.clone(),
        };
        let item = OneItemStateV1 {
            item_instance_id: uuid(0xee),
            world_id: uuid(0xee),
            definition: Some(typed),
            quantity: u32::MAX,
            lifecycle: u32::MAX,
        };
        let entry = OneItemContainerEntryV1 {
            parent_item_instance_id: uuid(0xee),
            placement_ordinal: u64::MAX,
        };
        let payload = OneItemTransactionV1 {
            interpretation_revision: u32::MAX,
            operation: Some(OneItemOperationV1::Transfer(OneItemTransferV1 {
                before: Some(item.clone()),
                after: Some(item),
                source: Some(OneItemGroundV1 {
                    world_id: uuid(0xee),
                    channel_id: uuid(0xee),
                    spatial_position: technical_bytes.clone(),
                    corpse_ref: technical_bytes.clone(),
                    map_revision: content.clone(),
                    content_revision: content.clone(),
                    native_room_placement_context: technical_bytes,
                    runtime_scope_ownership_generation: u64::MAX,
                }),
                destination: Some(OneItemInventoryV1 {
                    character_id: uuid(0xee),
                    expected_session_generation: u64::MAX,
                    expected_game_session_id: uuid(0xee),
                    expected_character_lease_generation: u64::MAX,
                    container_entry: Some(entry.clone()),
                    equipment_container_slot: true,
                }),
                cause: Some(OneItemTransferCauseV1 {
                    typed_cause: technical,
                    content_revision: content.clone(),
                    ruleset_revision: content.clone(),
                    sim_revision: content,
                    command_ref: Some(OneItemCommandRefV1 {
                        game_session_id: uuid(0xee),
                        command_id: u64::MAX,
                    }),
                }),
                receiver: Some(OneItemReceiverV1 {
                    item_instance_id: uuid(0xee),
                    position: Some(entry),
                    quantity_before: u32::MAX,
                    quantity_after: u32::MAX,
                }),
            })),
        };
        worst_case_envelope(encode_bounded(&payload, RL07_PAYLOAD_BYTES_MAX).unwrap())
    }

    #[test]
    fn worst_case_merge_shape_fits_the_registered_caps() {
        let (payload, envelope) = worst_case_top_up();
        // B3 §4.5: the merge shapes stay within 7,821 B of payload.
        assert!(payload <= 7_821, "payload {payload}");
        assert!(payload <= RL07_PAYLOAD_BYTES_MAX);
        assert!(envelope <= RL07_ENVELOPE_BYTES_MAX, "envelope {envelope}");
        assert_eq!((payload, envelope), (5_778, 6_816));
    }
}
