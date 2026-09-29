//! Registered DUR-03 `DECAY_RETIRE` durable-audit encoding (child D3-6;
//! decision `D3-CORPSE-CONTAINER-LOOT-WINDOW-DECAY-V1` §4.7/D136, DUR-03
//! §39.1/§39.4): event type 2 (`oteryn.events.v1.OneItemTransactionV1`,
//! operation `decay_retire`, oneof tag 5) in the normative ANL-01
//! `EventEnvelope`.
//!
//! One step retires exactly one live item — a corpse `ItemInstance` from its
//! own Ground, or one live entry of that corpse's container — to `RETIRED`,
//! quantity 0 and no location, under the one closed cause
//! `CorpseDecay { corpse_item, deadline }`. A corpse decays as N+1 such
//! events, never one multi-item event, so each fits the default
//! `DUR03-RL-01`/`-RL-06` one-item shape and no new resource row exists. The
//! event is server originated: it carries no session, CommandRef or
//! causation. Every registered bound is checked before encode allocation and
//! oversize input is rejected, never truncated.

use super::item_mint_audit::{
    self as mint, AuditError, EventEnvelopeV1, ITEM_LIFECYCLE_LIVE, OneItemGroundV1,
    OneItemOperationV1, OneItemStateV1, OneItemTransactionV1, RL07_ENVELOPE_BYTES_MAX,
    RL07_PAYLOAD_BYTES_MAX, TransactionEventRefV1, TransactionResourceUsage, check_content_key,
    check_definition, check_technical_bytes, check_uuid_v7, encode_bounded,
};
use super::item_transfer_audit::{ITEM_LIFECYCLE_RETIRED, OneItemCorpseSourceV1};
use prost::Message;
use sha2::{Digest, Sha256};

/// D135: a corpse decays `materialized_at + 60 s` after materialization; the
/// audited deadline is therefore never below this.
pub const CORPSE_DECAY_AFTER_MS: u64 = 60_000;

/// `CorpseDecay { corpse_item, deadline }` (DUR-03 §39.4).
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemCorpseDecayV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub corpse_item_instance_id: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub deadline_unix_ms: u64,
}

/// One `DECAY_RETIRE` step: exactly one of `ground` (the corpse's own step)
/// and `corpse_entry` (an entry's step) is the before-location.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemDecayRetireV1 {
    #[prost(message, optional, tag = "1")]
    pub before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub after: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "3")]
    pub ground: Option<OneItemGroundV1>,
    #[prost(message, optional, tag = "4")]
    pub corpse_entry: Option<OneItemCorpseSourceV1>,
    #[prost(message, optional, tag = "5")]
    pub cause: Option<OneItemCorpseDecayV1>,
    #[prost(uint64, tag = "6")]
    pub runtime_scope_ownership_generation: u64,
}

/// Which of the N+1 steps one event records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecayRetireShape {
    /// A live entry of the corpse's container retires (one of the N steps).
    Entry,
    /// The now-empty corpse itself retires from its Ground (the final step).
    Corpse,
}

/// DUR03-RL-01/-RL-06 defaults: one ItemInstance, its one location line (the
/// removal), one participant whose work is the participant, the removal and
/// the retirement, one event.
pub const DECAY_RETIRE_USAGE: TransactionResourceUsage = TransactionResourceUsage {
    touched_item_instances: 1,
    location_custody_lines: 1,
    value_lines: 0,
    transform_lines: 0,
    container_expansion: 0,
    participants: 1,
    effect_work_units: 3,
    events: 1,
};

fn check_state(value: &OneItemStateV1) -> Result<(), AuditError> {
    check_uuid_v7(&value.item_instance_id)?;
    check_uuid_v7(&value.world_id)?;
    check_definition(value.definition.as_ref())
}

fn check_ground(value: &OneItemGroundV1) -> Result<(), AuditError> {
    check_uuid_v7(&value.world_id)?;
    check_uuid_v7(&value.channel_id)?;
    check_technical_bytes(&value.spatial_position)?;
    check_technical_bytes(&value.corpse_ref)?;
    check_content_key(&value.map_revision)?;
    check_content_key(&value.content_revision)?;
    check_technical_bytes(&value.native_room_placement_context)?;
    if value.runtime_scope_ownership_generation == 0 {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

/// The corpse's live Ground: the scope authority of either step.
fn scope_ground(value: &OneItemDecayRetireV1) -> Result<&OneItemGroundV1, AuditError> {
    match (value.ground.as_ref(), value.corpse_entry.as_ref()) {
        (Some(ground), None) => Ok(ground),
        (None, Some(entry)) => entry.corpse_ground.as_ref().ok_or(AuditError::InvalidInput),
        _ => Err(AuditError::InvalidInput),
    }
}

/// Complete closed `DECAY_RETIRE` shape, conservation (the whole before
/// quantity retires to exactly 0) and every registered bound.
pub fn check_decay_retire(value: &OneItemDecayRetireV1) -> Result<DecayRetireShape, AuditError> {
    let before = value.before.as_ref().ok_or(AuditError::InvalidInput)?;
    let after = value.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let cause = value.cause.as_ref().ok_or(AuditError::InvalidInput)?;
    check_state(before)?;
    check_state(after)?;
    check_uuid_v7(&cause.corpse_item_instance_id)?;
    let ground = scope_ground(value)?;
    check_ground(ground)?;
    if before.item_instance_id != after.item_instance_id
        || before.world_id != after.world_id
        || before.definition != after.definition
        || before.lifecycle != ITEM_LIFECYCLE_LIVE
        || before.quantity == 0
        || after.lifecycle != ITEM_LIFECYCLE_RETIRED
        || after.quantity != 0
        || ground.world_id != before.world_id
        || cause.deadline_unix_ms < CORPSE_DECAY_AFTER_MS
        || value.runtime_scope_ownership_generation == 0
    {
        return Err(AuditError::InvalidInput);
    }
    let shape = match value.corpse_entry.as_ref() {
        None => {
            // The corpse's own step retires the corpse named by the cause.
            if before.item_instance_id != cause.corpse_item_instance_id {
                return Err(AuditError::InvalidInput);
            }
            DecayRetireShape::Corpse
        }
        Some(entry) => {
            // An entry's step: a direct entry of the cause's corpse, never
            // the corpse itself, at a GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX
            // ordinal.
            if entry.corpse_item_instance_id != cause.corpse_item_instance_id
                || entry.corpse_item_instance_id == before.item_instance_id
                || !(1..=mint::GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX)
                    .contains(&entry.placement_ordinal)
            {
                return Err(AuditError::InvalidInput);
            }
            DecayRetireShape::Entry
        }
    };
    DECAY_RETIRE_USAGE.check()?;
    Ok(shape)
}

/// RL-07 payload gate for `DECAY_RETIRE`: size before decode, canonical round
/// trip, closed shape and per-field bounds.
pub fn decode_decay_retire_payload(wire: &[u8]) -> Result<OneItemDecayRetireV1, AuditError> {
    if wire.len() > RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = OneItemTransactionV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire
        || value.interpretation_revision != mint::INTERPRETATION_REVISION
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::DecayRetire(retire)) = value.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_decay_retire(&retire)?;
    Ok(retire)
}

/// RL-07 envelope gate for `DECAY_RETIRE`: the shared envelope gate, the
/// payload gate, the envelope scope equal to the corpse's Ground and no
/// player session, command or causation.
pub fn decode_decay_retire_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, OneItemDecayRetireV1), AuditError> {
    let value = mint::decode_common_envelope(wire)?;
    let retire = decode_decay_retire_payload(&value.payload)?;
    let ground = scope_ground(&retire)?;
    let deadline = retire
        .cause
        .as_ref()
        .map(|cause| cause.deadline_unix_ms)
        .ok_or(AuditError::InvalidInput)?;
    if value.world_id.as_deref() != Some(ground.world_id.as_slice())
        || value.channel_id.as_deref() != Some(ground.channel_id.as_slice())
        || value.game_session_id.is_some()
        || value.command_id.is_some()
        || value.connection_generation.is_some()
        || value.causation.is_some()
        // The retirement is never earlier than the deadline that authorized it.
        || u64::try_from(value.occurred_at_unix_ms).map_or(true, |at| at < deadline)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((value, retire))
}

/// Envelope identity fixed before the commit attempt that can become ambiguous.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecayRetireEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: &'a str,
}

/// Build the exact immutable event bytes of one `DECAY_RETIRE` step;
/// re-decoded through both gates before they are returned.
pub fn encode_decay_retire_event(
    identity: DecayRetireEventIdentity<'_>,
    retire: OneItemDecayRetireV1,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    mint::check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    check_decay_retire(&retire)?;
    let ground = scope_ground(&retire)?;
    let (world_id, channel_id) = (ground.world_id.clone(), ground.channel_id.clone());
    let payload = encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::DecayRetire(retire)),
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
    let (decoded, _) = decode_decay_retire_envelope(&wire)?;
    if decoded != envelope {
        return Err(AuditError::InvalidInput);
    }
    Ok(wire)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::super::item_mint_audit::{
        CreatureDeathOccurrenceRefV1, LOOT_MINT_TYPED_CAUSE, OneItemMintV1, OneItemProvenanceV1,
        OneItemTypedDefinitionRevisionV1, RL01_TOUCHED_ITEM_INSTANCES_MAX,
        RL02_LOCATION_CUSTODY_LINES_MAX, RL06_EFFECT_WORK_UNITS_MAX, RL06_PARTICIPANTS_MAX,
        decode_payload,
    };
    use super::super::item_transfer_audit::{
        GROUND_PICKUP_TYPED_CAUSE, OneItemCommandRefV1, OneItemContainerEntryV1,
        OneItemInventoryV1, OneItemReceiverV1, OneItemTransferCauseV1, OneItemTransferV1,
        decode_transfer_payload,
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

    fn fixed(tag: u8) -> [u8; 16] {
        uuid(tag).try_into().unwrap()
    }

    const CORPSE: u8 = 60;
    const ENTRY: u8 = 9;
    const DEADLINE: u64 = 1_790_000_060_000;

    fn state(item: u8, quantity: u32, lifecycle: u32) -> OneItemStateV1 {
        OneItemStateV1 {
            item_instance_id: uuid(item),
            world_id: uuid(1),
            definition: Some(OneItemTypedDefinitionRevisionV1 {
                family: "ItemType".into(),
                production_key: "fixture:corpse-loot".into(),
                revision_ref: "rev-1".into(),
            }),
            quantity,
            lifecycle,
        }
    }

    fn ground() -> OneItemGroundV1 {
        OneItemGroundV1 {
            world_id: uuid(1),
            channel_id: uuid(2),
            spatial_position: vec![9, 9, 0],
            corpse_ref: uuid(3),
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            native_room_placement_context: uuid(6),
            runtime_scope_ownership_generation: 1,
        }
    }

    fn entry_step() -> OneItemDecayRetireV1 {
        OneItemDecayRetireV1 {
            before: Some(state(ENTRY, 3, ITEM_LIFECYCLE_LIVE)),
            after: Some(state(ENTRY, 0, ITEM_LIFECYCLE_RETIRED)),
            ground: None,
            corpse_entry: Some(OneItemCorpseSourceV1 {
                corpse_item_instance_id: uuid(CORPSE),
                placement_ordinal: 4,
                corpse_ground: Some(ground()),
            }),
            cause: Some(OneItemCorpseDecayV1 {
                corpse_item_instance_id: uuid(CORPSE),
                deadline_unix_ms: DEADLINE,
            }),
            runtime_scope_ownership_generation: 2,
        }
    }

    fn corpse_step() -> OneItemDecayRetireV1 {
        OneItemDecayRetireV1 {
            before: Some(state(CORPSE, 1, ITEM_LIFECYCLE_LIVE)),
            after: Some(state(CORPSE, 0, ITEM_LIFECYCLE_RETIRED)),
            ground: Some(ground()),
            corpse_entry: None,
            cause: Some(OneItemCorpseDecayV1 {
                corpse_item_instance_id: uuid(CORPSE),
                deadline_unix_ms: DEADLINE,
            }),
            runtime_scope_ownership_generation: 2,
        }
    }

    fn identity(occurred_at_unix_ms: i64) -> DecayRetireEventIdentity<'static> {
        DecayRetireEventIdentity {
            event_id: fixed(30),
            transaction_id: fixed(40),
            occurred_at_unix_ms,
            server_build_id: "oteryn-game-server/test",
        }
    }

    const AT_DEADLINE: i64 = 1_790_000_060_000;

    #[test]
    fn both_step_shapes_round_trip_and_other_gates_reject_them() {
        for (value, shape) in [
            (entry_step(), DecayRetireShape::Entry),
            (corpse_step(), DecayRetireShape::Corpse),
        ] {
            assert_eq!(check_decay_retire(&value), Ok(shape));
            let wire = encode_decay_retire_event(identity(AT_DEADLINE), value.clone()).unwrap();
            let (envelope, decoded) = decode_decay_retire_envelope(&wire).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(envelope.world_id.as_deref(), Some(uuid(1).as_slice()));
            assert_eq!(envelope.channel_id.as_deref(), Some(uuid(2).as_slice()));
            assert!(envelope.game_session_id.is_none() && envelope.causation.is_none());
            // The MINT and TRANSFER gates never admit a decay operation.
            assert_eq!(
                decode_payload(&envelope.payload).map(|_| ()),
                Err(AuditError::InvalidInput)
            );
            assert_eq!(
                decode_transfer_payload(&envelope.payload).map(|_| ()),
                Err(AuditError::InvalidInput)
            );
            // Operation tag 5 on the wire: field 5, length-delimited.
            assert_eq!(envelope.payload[2], (5 << 3) | 2);
        }
    }

    #[test]
    fn a_retirement_before_its_deadline_is_not_encodable() {
        assert!(encode_decay_retire_event(identity(AT_DEADLINE - 1), entry_step()).is_err());
        assert!(encode_decay_retire_event(identity(AT_DEADLINE), entry_step()).is_ok());
    }

    #[test]
    fn open_shapes_reject() {
        let bad: [fn(&mut OneItemDecayRetireV1); 14] = [
            // Both before-locations at once.
            |r| r.ground = Some(ground()),
            // Neither before-location.
            |r| r.corpse_entry = None,
            // An entry without the corpse's Ground.
            |r| r.corpse_entry.as_mut().unwrap().corpse_ground = None,
            // The entry names another corpse than the cause.
            |r| r.corpse_entry.as_mut().unwrap().corpse_item_instance_id = uuid(61),
            // The retired item is the entry's own parent.
            |r| r.corpse_entry.as_mut().unwrap().corpse_item_instance_id = uuid(ENTRY),
            // Ordinal zero and GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX + 1.
            |r| r.corpse_entry.as_mut().unwrap().placement_ordinal = 0,
            |r| r.corpse_entry.as_mut().unwrap().placement_ordinal = 17,
            // A partial reduction instead of a retirement to exactly 0.
            |r| r.after.as_mut().unwrap().quantity = 1,
            // Still live after.
            |r| r.after.as_mut().unwrap().lifecycle = ITEM_LIFECYCLE_LIVE,
            // Already retired before.
            |r| r.before.as_mut().unwrap().lifecycle = ITEM_LIFECYCLE_RETIRED,
            // Another item after than before.
            |r| r.after.as_mut().unwrap().item_instance_id = uuid(10),
            // No cause, or a deadline below materialized_at 0 + 60 s.
            |r| r.cause = None,
            |r| r.cause.as_mut().unwrap().deadline_unix_ms = CORPSE_DECAY_AFTER_MS - 1,
            // No fence generation.
            |r| r.runtime_scope_ownership_generation = 0,
        ];
        for (index, mutate) in bad.into_iter().enumerate() {
            let mut value = entry_step();
            mutate(&mut value);
            assert!(check_decay_retire(&value).is_err(), "case {index}");
            assert!(
                encode_decay_retire_event(identity(AT_DEADLINE), value).is_err(),
                "case {index}"
            );
        }
        // The corpse's own step retires exactly the cause's corpse.
        let mut value = corpse_step();
        value.cause.as_mut().unwrap().corpse_item_instance_id = uuid(61);
        assert!(check_decay_retire(&value).is_err());
    }

    #[test]
    fn envelope_scope_must_equal_the_corpse_ground_and_carry_no_command() {
        let wire = encode_decay_retire_event(identity(AT_DEADLINE), entry_step()).unwrap();
        let tamper: [fn(&mut EventEnvelopeV1); 5] = [
            |e| e.channel_id = Some(uuid(8)),
            |e| e.world_id = None,
            |e| e.command_id = Some(7),
            |e| e.game_session_id = Some(uuid(50)),
            |e| e.occurred_at_unix_ms = AT_DEADLINE - 1,
        ];
        for mutate in tamper {
            let mut envelope = EventEnvelopeV1::decode(wire.as_slice()).unwrap();
            mutate(&mut envelope);
            assert!(decode_decay_retire_envelope(&envelope.encode_to_vec()).is_err());
        }
    }

    #[test]
    fn usage_is_the_default_one_item_shape() {
        assert_eq!(DECAY_RETIRE_USAGE.touched_item_instances, 1);
        assert_eq!(DECAY_RETIRE_USAGE.participants, 1);
        const {
            assert!(DECAY_RETIRE_USAGE.touched_item_instances <= RL01_TOUCHED_ITEM_INSTANCES_MAX);
            assert!(DECAY_RETIRE_USAGE.location_custody_lines <= RL02_LOCATION_CUSTODY_LINES_MAX);
            assert!(DECAY_RETIRE_USAGE.participants <= RL06_PARTICIPANTS_MAX);
            assert!(DECAY_RETIRE_USAGE.effect_work_units <= RL06_EFFECT_WORK_UNITS_MAX);
        }
        assert_eq!(DECAY_RETIRE_USAGE.check(), Ok(()));
    }

    // ---- D3 widened fields round-trip losslessly against the Ground-only shapes.

    /// `OneItemMintV1` as it was before D3-2 (Ground destination only).
    #[derive(Clone, PartialEq, Eq, Message)]
    struct GroundOnlyMintV1 {
        #[prost(message, optional, tag = "1")]
        after: Option<OneItemStateV1>,
        #[prost(message, optional, tag = "2")]
        destination: Option<OneItemGroundV1>,
        #[prost(message, optional, tag = "3")]
        source: Option<OneItemProvenanceV1>,
        #[prost(bool, tag = "4")]
        before_semantically_absent: bool,
    }

    /// `OneItemTransferV1` as it was before D3-4 (Ground source only).
    #[derive(Clone, PartialEq, Eq, Message)]
    struct GroundOnlyTransferV1 {
        #[prost(message, optional, tag = "1")]
        before: Option<OneItemStateV1>,
        #[prost(message, optional, tag = "2")]
        after: Option<OneItemStateV1>,
        #[prost(message, optional, tag = "3")]
        source: Option<OneItemGroundV1>,
        #[prost(message, optional, tag = "4")]
        destination: Option<OneItemInventoryV1>,
        #[prost(message, optional, tag = "5")]
        cause: Option<OneItemTransferCauseV1>,
        #[prost(message, optional, tag = "6")]
        receiver: Option<OneItemReceiverV1>,
    }

    fn ground_mint() -> GroundOnlyMintV1 {
        GroundOnlyMintV1 {
            after: Some(state(ENTRY, 3, ITEM_LIFECYCLE_LIVE)),
            destination: Some(ground()),
            source: Some(OneItemProvenanceV1 {
                typed_cause: LOOT_MINT_TYPED_CAUSE.into(),
                death_occurrence: Some(CreatureDeathOccurrenceRefV1 {
                    world_id: uuid(1),
                    channel_id: uuid(2),
                    scope_ownership_generation: 1,
                    actor_local_id: 7,
                    actor_local_generation: 1,
                }),
                loot_table: Some(OneItemTypedDefinitionRevisionV1 {
                    family: "LootTable".into(),
                    production_key: "fixture:loot.alpha".into(),
                    revision_ref: "loot-r1".into(),
                }),
                loot_purpose_key: "fixture:purpose.drop".into(),
                draw_ordinal: 1,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            }),
            before_semantically_absent: true,
        }
    }

    fn ground_transfer() -> GroundOnlyTransferV1 {
        GroundOnlyTransferV1 {
            before: Some(state(ENTRY, 3, ITEM_LIFECYCLE_LIVE)),
            after: Some(state(ENTRY, 3, ITEM_LIFECYCLE_LIVE)),
            source: Some(ground()),
            destination: Some(OneItemInventoryV1 {
                character_id: uuid(41),
                expected_session_generation: 1,
                expected_game_session_id: uuid(50),
                expected_character_lease_generation: 1,
                container_entry: Some(OneItemContainerEntryV1 {
                    parent_item_instance_id: uuid(20),
                    placement_ordinal: 1,
                }),
                equipment_container_slot: false,
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
            receiver: None,
        }
    }

    #[test]
    fn widened_mint_destination_round_trips_against_the_ground_only_shape() {
        // Ground-only bytes decode into the widened message and re-encode
        // byte-identically, with field 5 absent; and back again.
        let legacy = ground_mint().encode_to_vec();
        let widened = OneItemMintV1::decode(legacy.as_slice()).unwrap();
        assert!(widened.corpse_container_entry.is_none());
        assert_eq!(widened.encode_to_vec(), legacy);
        assert_eq!(
            GroundOnlyMintV1::decode(widened.encode_to_vec().as_slice()).unwrap(),
            ground_mint()
        );
        // The corpse-loot form (field 5 instead of field 2) round-trips
        // losslessly through the widened message ...
        let mut corpse_loot = widened;
        corpse_loot.destination = None;
        corpse_loot.corpse_container_entry = Some(OneItemContainerEntryV1 {
            parent_item_instance_id: uuid(CORPSE),
            placement_ordinal: 2,
        });
        let wire = corpse_loot.encode_to_vec();
        assert_eq!(OneItemMintV1::decode(wire.as_slice()).unwrap(), corpse_loot);
        // ... while a Ground-only reader cannot reproduce it, so the
        // canonical round trip of such a reader fails closed instead of
        // silently dropping the destination.
        let dropped = GroundOnlyMintV1::decode(wire.as_slice()).unwrap();
        assert_ne!(dropped.encode_to_vec(), wire);
    }

    #[test]
    fn widened_transfer_source_round_trips_against_the_ground_only_shape() {
        let legacy = ground_transfer().encode_to_vec();
        let widened = OneItemTransferV1::decode(legacy.as_slice()).unwrap();
        assert!(widened.corpse_source.is_none());
        assert_eq!(widened.encode_to_vec(), legacy);
        assert_eq!(
            GroundOnlyTransferV1::decode(widened.encode_to_vec().as_slice()).unwrap(),
            ground_transfer()
        );
        let mut corpse_sourced = widened;
        let corpse_ground = corpse_sourced.source.take();
        corpse_sourced.corpse_source = Some(Box::new(OneItemCorpseSourceV1 {
            corpse_item_instance_id: uuid(CORPSE),
            placement_ordinal: 2,
            corpse_ground,
        }));
        let wire = corpse_sourced.encode_to_vec();
        assert_eq!(
            OneItemTransferV1::decode(wire.as_slice()).unwrap(),
            corpse_sourced
        );
        let dropped = GroundOnlyTransferV1::decode(wire.as_slice()).unwrap();
        assert_ne!(dropped.encode_to_vec(), wire);
    }
}
