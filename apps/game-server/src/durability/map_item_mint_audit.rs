//! Registered DUR-03 map-item MINT durable-audit encoding: event type 2
//! (`oteryn.events.v1.OneItemTransactionV1`, operation `map_item_mint`,
//! oneof tag 8) in the normative ANL-01 `EventEnvelope`, for the
//! MAP-OVERLAY-1b materialization of an eligible base-map entry (ADR-0021
//! §4.4): one fresh live item placed on Ground at the entry's own tile, in
//! the Channel that takes it.
//!
//! The cause is the actual player pickup CommandRef and the exact base-map
//! entry: World, Channel, base bundle digest, `placement_key` and reset
//! epoch. Every registered bound is checked before encode allocation and
//! oversize input is rejected, never truncated.

use super::item_mint_audit::{
    self as mint, AuditError, CausationRefV1, CausationV1, EnvelopeCommandRefV1, EventEnvelopeV1,
    ITEM_LIFECYCLE_LIVE, OneItemGroundV1, OneItemOperationV1, OneItemStateV1, OneItemTransactionV1,
    RL07_ENVELOPE_BYTES_MAX, RL07_PAYLOAD_BYTES_MAX, TransactionEventRefV1,
    TransactionResourceUsage, check_content_key, check_definition, check_technical_bytes,
    check_technical_text, check_uuid_v7, encode_bounded,
};
use super::item_transfer_audit::{GAMEITEM01_STACK_QUANTITY_MAX, OneItemCommandRefV1};
use prost::Message;
use sha2::{Digest, Sha256};

/// Typed cause of a map-item MINT (D40).
pub const MAP_ITEM_MINT_TYPED_CAUSE: &str = "map_item_mint";

/// The Ground `corpse_ref` of every map-item MINT: a fixed marker, since the
/// item has no corpse and the column is never empty.
pub const MAP_ITEM_CORPSE_REF: &[u8] = b"map-item-materialization-v1";

/// Width of a base bundle digest (SHA-256).
pub const MAP_ITEM_BUNDLE_DIGEST_BYTES: usize = 32;

/// Width of a Ground `spatial_position`: x i32, y i32, floor i16, big endian.
pub const MAP_ITEM_SPATIAL_POSITION_BYTES: usize = 10;

/// The base-map entry one materialization takes, once per Channel and reset
/// epoch.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemMapItemMaterializationV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub channel_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub base_bundle_digest: Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub placement_key: u64,
    #[prost(uint64, tag = "5")]
    pub reset_epoch: u64,
}

/// Map-item MINT cause. Same field numbers as `OneItemTransferCauseV1` for
/// the shared fields; field 10 is the base-map entry.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemMapItemMintCauseV1 {
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
    #[prost(message, optional, tag = "10")]
    pub materialization: Option<OneItemMapItemMaterializationV1>,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemMapItemMintV1 {
    #[prost(message, optional, tag = "1")]
    pub after: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub destination: Option<OneItemGroundV1>,
    #[prost(message, optional, tag = "3")]
    pub source: Option<OneItemMapItemMintCauseV1>,
    #[prost(bool, tag = "4")]
    pub before_semantically_absent: bool,
    /// The connection generation of the commanding session.
    #[prost(uint64, tag = "5")]
    pub connection_generation: u64,
}

/// One fresh ItemInstance, one new Ground line, one participant whose work
/// is the participant, the Ground placement and the materialization record,
/// one event.
pub const MAP_ITEM_MINT_USAGE: TransactionResourceUsage = TransactionResourceUsage {
    touched_item_instances: 1,
    location_custody_lines: 1,
    value_lines: 0,
    transform_lines: 0,
    container_expansion: 0,
    participants: 1,
    effect_work_units: 3,
    events: 1,
};

/// `"sha256:"` and the lowercase hex of a base bundle digest: the
/// `map_revision` of a Ground item of that bundle.
pub fn map_revision_of(digest: &[u8]) -> String {
    let mut revision = String::with_capacity(7 + 2 * digest.len());
    revision.push_str("sha256:");
    for byte in digest {
        revision.push_str(&format!("{byte:02x}"));
    }
    revision
}

/// The Ground `spatial_position` of a native tile.
pub fn map_item_spatial_position(x: u16, y: u16, floor: i8) -> Vec<u8> {
    let mut position = Vec::with_capacity(MAP_ITEM_SPATIAL_POSITION_BYTES);
    position.extend_from_slice(&i32::from(x).to_be_bytes());
    position.extend_from_slice(&i32::from(y).to_be_bytes());
    position.extend_from_slice(&i16::from(floor).to_be_bytes());
    position
}

/// Whether `spatial_position` is the exact tile `placement_key` names.
pub fn position_names_placement(spatial_position: &[u8], placement_key: u64) -> bool {
    let Ok(bytes) = <[u8; MAP_ITEM_SPATIAL_POSITION_BYTES]>::try_from(spatial_position) else {
        return false;
    };
    let x = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let y = i32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let floor = i16::from_be_bytes([bytes[8], bytes[9]]);
    let (Ok(x), Ok(y), Ok(floor)) = (u16::try_from(x), u16::try_from(y), i8::try_from(floor))
    else {
        return false;
    };
    oteryn_world_bundle::bundle::placement_key(floor, x, y, (placement_key & 0xff) as u8)
        == Some(placement_key)
}

/// Complete closed MAP-OVERLAY-1b shape and every registered per-field bound.
pub fn check_map_item_mint(value: &OneItemMapItemMintV1) -> Result<(), AuditError> {
    let after = value.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let ground = value.destination.as_ref().ok_or(AuditError::InvalidInput)?;
    let source = value.source.as_ref().ok_or(AuditError::InvalidInput)?;
    let command = source
        .command_ref
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    let entry = source
        .materialization
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
    check_content_key(&source.content_revision)?;
    check_content_key(&source.ruleset_revision)?;
    check_content_key(&source.sim_revision)?;
    check_uuid_v7(&command.game_session_id)?;
    check_uuid_v7(&entry.world_id)?;
    check_uuid_v7(&entry.channel_id)?;
    if !value.before_semantically_absent
        || after.lifecycle != ITEM_LIFECYCLE_LIVE
        || after.quantity == 0
        || after.quantity > GAMEITEM01_STACK_QUANTITY_MAX
        || source.typed_cause != MAP_ITEM_MINT_TYPED_CAUSE
        || command.command_id == 0
        || value.connection_generation == 0
        || entry.base_bundle_digest.len() != MAP_ITEM_BUNDLE_DIGEST_BYTES
        || after.world_id != entry.world_id
        || ground.world_id != entry.world_id
        || ground.channel_id != entry.channel_id
        || ground.corpse_ref != MAP_ITEM_CORPSE_REF
        || ground.runtime_scope_ownership_generation == 0
        || ground.native_room_placement_context != entry.base_bundle_digest
        || ground.map_revision != map_revision_of(&entry.base_bundle_digest)
        || !position_names_placement(&ground.spatial_position, entry.placement_key)
    {
        return Err(AuditError::InvalidInput);
    }
    MAP_ITEM_MINT_USAGE.check()
}

pub fn decode_map_item_mint_payload(wire: &[u8]) -> Result<OneItemMapItemMintV1, AuditError> {
    if wire.len() > RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = OneItemTransactionV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire
        || value.interpretation_revision != mint::INTERPRETATION_REVISION
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::MapItemMint(minted)) = value.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_map_item_mint(&minted)?;
    Ok(minted)
}

/// Envelope gate: the common RL-07 gate, then the World and Channel of the
/// materialization, and the command, session, connection generation and
/// causation of the payload cause.
pub fn decode_map_item_mint_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, OneItemMapItemMintV1), AuditError> {
    let value = mint::decode_common_envelope(wire)?;
    let minted = decode_map_item_mint_payload(&value.payload)?;
    let source = minted.source.as_ref().ok_or(AuditError::InvalidInput)?;
    let command = source
        .command_ref
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    let entry = source
        .materialization
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    let envelope_command = CausationRefV1 {
        cause: Some(CausationV1::Command(EnvelopeCommandRefV1 {
            game_session_id: command.game_session_id.clone(),
            command_id: command.command_id,
        })),
    };
    if value.world_id.as_deref() != Some(entry.world_id.as_slice())
        || value.channel_id.as_deref() != Some(entry.channel_id.as_slice())
        || value.game_session_id.as_deref() != Some(command.game_session_id.as_slice())
        || value.command_id != Some(command.command_id)
        || value.connection_generation != Some(minted.connection_generation)
        || value.causation.as_ref() != Some(&envelope_command)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((value, minted))
}

/// Envelope identity fixed before the first commit attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapItemMintEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: &'a str,
}

/// Build the exact immutable event bytes for one map-item MINT. The returned
/// envelope is re-decoded through the registered gates first.
pub fn encode_map_item_mint_event(
    identity: MapItemMintEventIdentity<'_>,
    minted: OneItemMapItemMintV1,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    check_map_item_mint(&minted)?;
    let source = minted.source.as_ref().ok_or(AuditError::InvalidInput)?;
    let command = source.command_ref.clone().ok_or(AuditError::InvalidInput)?;
    let entry = source
        .materialization
        .clone()
        .ok_or(AuditError::InvalidInput)?;
    let connection_generation = minted.connection_generation;
    let payload = encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::MapItemMint(minted)),
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
        world_id: Some(entry.world_id),
        channel_id: Some(entry.channel_id),
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
    let (decoded, _) = decode_map_item_mint_envelope(&wire)?;
    if decoded != envelope {
        return Err(AuditError::InvalidInput);
    }
    Ok(wire)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::super::item_mint_audit::{
        OneItemTypedDefinitionRevisionV1, RL07_CONTENT_KEY_BYTES_MAX,
        RL07_TECHNICAL_FIELD_BYTES_MAX, decode_envelope, decode_payload,
        tests::worst_case_envelope,
    };
    use super::super::item_transfer_audit::{decode_transfer_envelope, decode_transfer_payload};
    use super::super::reward_claim_mint_audit::{
        decode_reward_claim_mint_envelope, decode_reward_claim_mint_payload,
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

    const DIGEST: [u8; 32] = [0x3c; 32];
    const KEY: u64 = 100 << 32 | 200 << 16 | 7 << 8 | 2;

    fn minted() -> OneItemMapItemMintV1 {
        OneItemMapItemMintV1 {
            after: Some(OneItemStateV1 {
                item_instance_id: uuid(9),
                world_id: uuid(1),
                definition: Some(OneItemTypedDefinitionRevisionV1 {
                    family: "Item".into(),
                    production_key: "fixture:map.apple".into(),
                    revision_ref: "definition-r1".into(),
                }),
                quantity: 3,
                lifecycle: ITEM_LIFECYCLE_LIVE,
            }),
            destination: Some(OneItemGroundV1 {
                world_id: uuid(1),
                channel_id: uuid(2),
                spatial_position: map_item_spatial_position(100, 200, -7),
                corpse_ref: MAP_ITEM_CORPSE_REF.to_vec(),
                map_revision: map_revision_of(&DIGEST),
                content_revision: "content-1".into(),
                native_room_placement_context: DIGEST.to_vec(),
                runtime_scope_ownership_generation: 4,
            }),
            source: Some(OneItemMapItemMintCauseV1 {
                typed_cause: MAP_ITEM_MINT_TYPED_CAUSE.into(),
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
                command_ref: Some(OneItemCommandRefV1 {
                    game_session_id: uuid(50),
                    command_id: 7,
                }),
                materialization: Some(OneItemMapItemMaterializationV1 {
                    world_id: uuid(1),
                    channel_id: uuid(2),
                    base_bundle_digest: DIGEST.to_vec(),
                    placement_key: KEY,
                    reset_epoch: 0,
                }),
            }),
            before_semantically_absent: true,
            connection_generation: 5,
        }
    }

    fn identity() -> MapItemMintEventIdentity<'static> {
        MapItemMintEventIdentity {
            event_id: fixed(30),
            transaction_id: fixed(31),
            occurred_at_unix_ms: 1_700_000_000_000,
            server_build_id: "test-build",
        }
    }

    fn cause(value: &mut OneItemMapItemMintV1) -> &mut OneItemMapItemMintCauseV1 {
        value.source.as_mut().unwrap()
    }

    fn entry(value: &mut OneItemMapItemMintV1) -> &mut OneItemMapItemMaterializationV1 {
        cause(value).materialization.as_mut().unwrap()
    }

    fn ground(value: &mut OneItemMapItemMintV1) -> &mut OneItemGroundV1 {
        value.destination.as_mut().unwrap()
    }

    #[test]
    fn map_item_mint_round_trips_with_its_full_cause() {
        let wire = encode_map_item_mint_event(identity(), minted()).unwrap();
        let (envelope, decoded) = decode_map_item_mint_envelope(&wire).unwrap();
        assert_eq!(decoded, minted());
        assert_eq!(envelope.world_id, Some(uuid(1)));
        assert_eq!(envelope.channel_id, Some(uuid(2)));
        assert_eq!(envelope.game_session_id, Some(uuid(50)));
        assert_eq!(envelope.command_id, Some(7));
        assert_eq!(envelope.connection_generation, Some(5));
        // Deterministic: the same frozen inputs give the same exact bytes.
        assert_eq!(
            encode_map_item_mint_event(identity(), minted()).unwrap(),
            wire
        );
        // Every other operation gate rejects the map-item operation.
        assert!(decode_payload(&envelope.payload).is_err());
        assert!(decode_transfer_payload(&envelope.payload).is_err());
        assert!(decode_reward_claim_mint_payload(&envelope.payload).is_err());
        assert!(decode_envelope(&wire).is_err());
        assert!(decode_transfer_envelope(&wire).is_err());
        assert!(decode_reward_claim_mint_envelope(&wire).is_err());
    }

    #[test]
    fn map_item_mint_refuses_a_missing_or_extra_cause_field() {
        let wire = encode_map_item_mint_event(identity(), minted()).unwrap();
        let (envelope, _) = decode_map_item_mint_envelope(&wire).unwrap();
        // Missing: the cause without its materialization re-encodes shorter
        // and is refused by the closed shape.
        let mut missing = minted();
        cause(&mut missing).materialization = None;
        let payload = OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::MapItemMint(missing)),
        }
        .encode_to_vec();
        assert_eq!(
            decode_map_item_mint_payload(&payload),
            Err(AuditError::InvalidInput)
        );
        // Extra: an unknown field inside the cause (tag 11, varint 1) is
        // dropped by the decoder, so only the canonical re-encode equality
        // refuses it. The hand-built layout without it is the exact payload.
        let build = |extra: &[u8]| {
            let mut value = minted();
            let mut cause_bytes = cause(&mut value).encode_to_vec();
            cause_bytes.extend_from_slice(extra);
            let mut operation = Vec::new();
            for (tag, bytes) in [
                (1_u8, value.after.as_ref().unwrap().encode_to_vec()),
                (2, value.destination.as_ref().unwrap().encode_to_vec()),
                (3, cause_bytes),
            ] {
                operation.push(tag << 3 | 2);
                prost::encoding::encode_varint(bytes.len() as u64, &mut operation);
                operation.extend_from_slice(&bytes);
            }
            // before_semantically_absent = true, connection_generation = 5.
            operation.extend_from_slice(&[0x20, 0x01, 0x28, 0x05]);
            let mut payload = vec![0x08];
            prost::encoding::encode_varint(u64::from(mint::INTERPRETATION_REVISION), &mut payload);
            payload.push(8 << 3 | 2);
            prost::encoding::encode_varint(operation.len() as u64, &mut payload);
            payload.extend_from_slice(&operation);
            payload
        };
        assert_eq!(build(&[]), envelope.payload);
        let extra = build(&[0x58, 0x01]);
        assert_eq!(
            OneItemTransactionV1::decode(extra.as_slice())
                .unwrap()
                .encode_to_vec(),
            envelope.payload
        );
        assert_eq!(
            decode_map_item_mint_payload(&extra),
            Err(AuditError::InvalidInput)
        );
    }

    #[test]
    fn map_item_mint_rejects_every_open_shape() {
        type Mutation = Box<dyn Fn(&mut OneItemMapItemMintV1)>;
        let cases: Vec<(&str, Mutation)> = vec![
            (
                "prior item",
                Box::new(|v| v.before_semantically_absent = false),
            ),
            (
                "zero quantity",
                Box::new(|v| v.after.as_mut().unwrap().quantity = 0),
            ),
            (
                "stack above 100",
                Box::new(|v| v.after.as_mut().unwrap().quantity = 101),
            ),
            (
                "retired",
                Box::new(|v| v.after.as_mut().unwrap().lifecycle = 2),
            ),
            ("no after", Box::new(|v| v.after = None)),
            ("no Ground", Box::new(|v| v.destination = None)),
            ("no cause", Box::new(|v| v.source = None)),
            (
                "other cause",
                Box::new(|v| cause(v).typed_cause = "loot_mint".into()),
            ),
            ("no command", Box::new(|v| cause(v).command_ref = None)),
            (
                "zero command",
                Box::new(|v| cause(v).command_ref.as_mut().unwrap().command_id = 0),
            ),
            (
                "zero connection generation",
                Box::new(|v| v.connection_generation = 0),
            ),
            (
                "no materialization",
                Box::new(|v| cause(v).materialization = None),
            ),
            (
                "short digest",
                Box::new(|v| entry(v).base_bundle_digest = vec![0x3c; 31]),
            ),
            (
                "item of another World",
                Box::new(|v| v.after.as_mut().unwrap().world_id = uuid(3)),
            ),
            (
                "Ground of another World",
                Box::new(|v| ground(v).world_id = uuid(3)),
            ),
            (
                "Ground of another Channel",
                Box::new(|v| ground(v).channel_id = uuid(3)),
            ),
            (
                "corpse marker",
                Box::new(|v| ground(v).corpse_ref = b"corpse".to_vec()),
            ),
            (
                "zero scope generation",
                Box::new(|v| ground(v).runtime_scope_ownership_generation = 0),
            ),
            (
                "placement context of another bundle",
                Box::new(|v| ground(v).native_room_placement_context = vec![0x3d; 32]),
            ),
            (
                "map revision of another bundle",
                Box::new(|v| ground(v).map_revision = map_revision_of(&[0x3d; 32])),
            ),
            (
                "another tile",
                Box::new(|v| ground(v).spatial_position = map_item_spatial_position(101, 200, -7)),
            ),
            (
                "another floor",
                Box::new(|v| ground(v).spatial_position = map_item_spatial_position(100, 200, -6)),
            ),
            (
                "ordinal beyond the tile reach",
                Box::new(|v| entry(v).placement_key = KEY & !0xff | 64),
            ),
            (
                "key above 48 bits",
                Box::new(|v| entry(v).placement_key = KEY | 1 << 48),
            ),
            (
                "position of 11 bytes",
                Box::new(|v| ground(v).spatial_position.push(0)),
            ),
            (
                "oversize content revision",
                Box::new(|v| {
                    cause(v).content_revision = "k".repeat(RL07_CONTENT_KEY_BYTES_MAX + 1);
                }),
            ),
        ];
        for (label, mutate) in cases {
            let mut value = minted();
            mutate(&mut value);
            assert!(check_map_item_mint(&value).is_err(), "{label}");
            assert!(
                encode_map_item_mint_event(identity(), value).is_err(),
                "{label}"
            );
        }
        assert!(check_map_item_mint(&minted()).is_ok());
        // Reset epoch zero and the top floor are admitted.
        let mut surface = minted();
        ground(&mut surface).spatial_position = map_item_spatial_position(100, 200, 0);
        entry(&mut surface).placement_key = 100 << 32 | 200 << 16 | 2;
        entry(&mut surface).reset_epoch = u64::MAX;
        assert!(check_map_item_mint(&surface).is_ok());
    }

    #[test]
    fn map_item_mint_envelope_binds_the_materialization_scope() {
        let wire = encode_map_item_mint_event(identity(), minted()).unwrap();
        let (envelope, _) = decode_map_item_mint_envelope(&wire).unwrap();
        type Mutation = Box<dyn Fn(&mut EventEnvelopeV1)>;
        let cases: Vec<(&str, Mutation)> = vec![
            ("World", Box::new(|e| e.world_id = Some(uuid(3)))),
            ("Channel", Box::new(|e| e.channel_id = Some(uuid(3)))),
            ("no Channel", Box::new(|e| e.channel_id = None)),
            ("session", Box::new(|e| e.game_session_id = Some(uuid(51)))),
            ("command", Box::new(|e| e.command_id = Some(8))),
            (
                "connection generation",
                Box::new(|e| e.connection_generation = Some(6)),
            ),
            ("causation", Box::new(|e| e.causation = None)),
        ];
        for (label, mutate) in cases {
            let mut value = envelope.clone();
            mutate(&mut value);
            assert!(
                decode_map_item_mint_envelope(&value.encode_to_vec()).is_err(),
                "{label}"
            );
        }
    }

    #[test]
    fn map_item_mint_worst_case_fits_the_registered_caps() {
        let content = "k".repeat(RL07_CONTENT_KEY_BYTES_MAX);
        let technical = "t".repeat(RL07_TECHNICAL_FIELD_BYTES_MAX);
        let technical_bytes = vec![0xa5; RL07_TECHNICAL_FIELD_BYTES_MAX];
        let payload = OneItemTransactionV1 {
            interpretation_revision: u32::MAX,
            operation: Some(OneItemOperationV1::MapItemMint(OneItemMapItemMintV1 {
                after: Some(OneItemStateV1 {
                    item_instance_id: uuid(0xee),
                    world_id: uuid(0xee),
                    definition: Some(OneItemTypedDefinitionRevisionV1 {
                        family: technical.clone(),
                        production_key: content.clone(),
                        revision_ref: content.clone(),
                    }),
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
                source: Some(OneItemMapItemMintCauseV1 {
                    typed_cause: technical,
                    content_revision: content.clone(),
                    ruleset_revision: content.clone(),
                    sim_revision: content,
                    command_ref: Some(OneItemCommandRefV1 {
                        game_session_id: uuid(0xee),
                        command_id: u64::MAX,
                    }),
                    materialization: Some(OneItemMapItemMaterializationV1 {
                        world_id: uuid(0xee),
                        channel_id: uuid(0xee),
                        base_bundle_digest: vec![0xee; MAP_ITEM_BUNDLE_DIGEST_BYTES],
                        placement_key: u64::MAX,
                        reset_epoch: u64::MAX,
                    }),
                }),
                before_semantically_absent: true,
                connection_generation: u64::MAX,
            })),
        };
        let (payload, envelope) =
            worst_case_envelope(encode_bounded(&payload, RL07_PAYLOAD_BYTES_MAX).unwrap());
        assert!(payload <= RL07_PAYLOAD_BYTES_MAX, "payload {payload}");
        assert!(envelope <= RL07_ENVELOPE_BYTES_MAX, "envelope {envelope}");
        assert_eq!((payload, envelope), (4514, 5552));
    }

    #[test]
    fn map_item_mint_usage_fits_the_registered_rows() {
        assert!(MAP_ITEM_MINT_USAGE.check().is_ok());
    }
}
