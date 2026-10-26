//! DUR-03 shapes of an item use and its durable-audit encoding (ITEM-USE-1; decision
//! `ITEM-USE-0` §4, DUR-03 §15, §39.1 and §39.3 item-use amendment): event type 2
//! (`oteryn.events.v1.OneItemTransactionV1`, operation `item_use`, oneof tag 10) in the
//! normative ANL-01 `EventEnvelope`.
//!
//! One event per use of one consumable unit (food or potion), under the closed
//! `ItemUseCause { Food, Potion }` whose identity is the using command's CommandRef:
//!
//! - **burn** (food, and a potion without a flask): exactly one unit leaves the used stack
//!   `S`, which keeps its identity or retires at zero (§11.1, §11.5);
//! - **flask** (a potion whose content names an empty flask): the same unit leaves `S` and
//!   one flask unit is added, as a quantity adjustment of a compatible stack `E` of the main
//!   backpack (§11.1) or as a fresh flask item `F` in a new entry (§11.3). One TRANSFORM line
//!   at unit level, at most two items touched, nothing minted;
//! - **flask missing** (a potion whose flask has no room, `PARITY_PENDING` until
//!   ITEM-MOVE-2b): the burn shape plus the recorded missing flask.
//!
//! The event is player originated and carries the session, the CommandRef and the rule 2
//! fence exactly as a TRANSFER does. Every bound is checked before encode allocation and
//! oversize input is rejected, never truncated.

use super::item_mint_audit::{
    self as mint, AuditError, EventEnvelopeV1, ITEM_LIFECYCLE_LIVE, OneItemOperationV1,
    OneItemStateV1, OneItemTransactionV1, TransactionEventRefV1, check_content_key,
    check_definition, check_uuid_v7, encode_bounded,
};
use super::item_transfer_audit::{
    GAMEITEM01_STACK_QUANTITY_MAX, ITEM_LIFECYCLE_RETIRED, OneItemCommandRefV1,
    OneItemContainerEntryV1,
};
use prost::Message;
use sha2::{Digest, Sha256};

// Item-use rows (decision §4.4; RESOURCE_LIMITS_REGISTRY.json `DUR03-RL-*-ITEM-USE*`).
pub const ITEM_USE_RL01_TOUCHED_ITEM_INSTANCES_MAX: u64 = 2;
pub const ITEM_USE_RL02_LOCATION_CUSTODY_LINES_MAX: u64 = 2;
pub const ITEM_USE_RL04_TRANSFORM_LINES_MAX: u64 = 1;
pub const ITEM_USE_RL06_PARTICIPANTS_MAX: u64 = 2;
pub const ITEM_USE_RL06_EFFECT_WORK_UNITS_MAX: u64 = 6;
pub const ITEM_USE_RL07_EVENTS_MAX: u64 = 1;
/// Measured worst cases (`DUR03-RL-07-ITEM-USE-PAYLOAD-BYTES` / `-ENVELOPE-BYTES`).
pub const ITEM_USE_RL07_PAYLOAD_BYTES_MAX: usize = 5_366;
pub const ITEM_USE_RL07_ENVELOPE_BYTES_MAX: usize = 5_754;

/// `OneItemUseCauseV1.kind` (closed `ItemUseCause`).
pub const ITEM_USE_CAUSE_FOOD: u32 = 1;
pub const ITEM_USE_CAUSE_POTION: u32 = 2;

/// One consumable unit leaves the stack.
const UNITS_PER_USE: u32 = 1;

/// Closed `ItemUseCause` and the interpretation revisions of the use.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemUseCauseV1 {
    #[prost(uint32, tag = "1")]
    pub kind: u32,
    #[prost(string, tag = "6")]
    pub content_revision: String,
    #[prost(string, tag = "7")]
    pub ruleset_revision: String,
    #[prost(string, tag = "8")]
    pub sim_revision: String,
    #[prost(message, optional, tag = "9")]
    pub command_ref: Option<OneItemCommandRefV1>,
}

/// The flask unit of a potion use: a stack `E` growing by one (`before` present) or a fresh
/// item `F` of quantity 1 (`before` absent) in a new entry.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemUseFlaskV1 {
    #[prost(message, optional, tag = "1")]
    pub before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub after: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "3")]
    pub entry: Option<OneItemContainerEntryV1>,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemUseV1 {
    #[prost(message, optional, tag = "1")]
    pub source_before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub source_after: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "3")]
    pub source_entry: Option<OneItemContainerEntryV1>,
    #[prost(message, optional, tag = "4")]
    pub cause: Option<OneItemUseCauseV1>,
    #[prost(bytes = "vec", tag = "5")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "6")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "7")]
    pub channel_id: Vec<u8>,
    #[prost(uint64, tag = "8")]
    pub runtime_scope_ownership_generation: u64,
    #[prost(uint64, tag = "9")]
    pub expected_session_generation: u64,
    #[prost(bytes = "vec", tag = "10")]
    pub expected_game_session_id: Vec<u8>,
    #[prost(message, optional, tag = "11")]
    pub flask: Option<OneItemUseFlaskV1>,
    #[prost(bool, tag = "12")]
    pub flask_missing: bool,
}

/// The shapes a use admits (decision §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemUseShape {
    /// Food, or a potion without a flask: one unit burns.
    Burn,
    /// A potion whose flask has no room: the burn shape plus the missing flask.
    BurnFlaskMissing,
    /// The flask unit grows a compatible stack of the main backpack.
    FlaskStack,
    /// The flask unit is a fresh item in a new entry.
    FlaskNew,
}

/// Per-transaction counts of a use shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemUseUsage {
    pub touched_item_instances: u64,
    pub location_custody_lines: u64,
    pub transform_lines: u64,
    pub participants: u64,
    pub effect_work_units: u64,
    pub events: u64,
}

impl ItemUseUsage {
    /// The registered ceilings: the flask shape retiring the used stack into a new entry.
    pub const CEILING: Self = Self {
        touched_item_instances: ITEM_USE_RL01_TOUCHED_ITEM_INSTANCES_MAX,
        location_custody_lines: ITEM_USE_RL02_LOCATION_CUSTODY_LINES_MAX,
        transform_lines: ITEM_USE_RL04_TRANSFORM_LINES_MAX,
        participants: ITEM_USE_RL06_PARTICIPANTS_MAX,
        effect_work_units: ITEM_USE_RL06_EFFECT_WORK_UNITS_MAX,
        events: ITEM_USE_RL07_EVENTS_MAX,
    };

    /// `self` against the rows, before any write.
    pub fn check(&self) -> Result<(), AuditError> {
        let ceiling = Self::CEILING;
        if self.touched_item_instances > ceiling.touched_item_instances
            || self.location_custody_lines > ceiling.location_custody_lines
            || self.transform_lines > ceiling.transform_lines
            || self.participants > ceiling.participants
            || self.effect_work_units > ceiling.effect_work_units
            || self.events > ceiling.events
        {
            return Err(AuditError::CapacityExceeded);
        }
        Ok(())
    }
}

impl ItemUseShape {
    /// The shape's usage. `source_retires` adds the removal of the used stack's entry; the
    /// work is the participants, one write per touched item, the TRANSFORM and the record.
    #[must_use]
    pub const fn usage(self, source_retires: bool) -> ItemUseUsage {
        let source_line = source_retires as u64;
        match self {
            Self::Burn | Self::BurnFlaskMissing => ItemUseUsage {
                touched_item_instances: 1,
                location_custody_lines: source_line,
                transform_lines: 0,
                participants: 1,
                effect_work_units: 3,
                events: 1,
            },
            Self::FlaskStack => ItemUseUsage {
                touched_item_instances: 2,
                location_custody_lines: source_line,
                transform_lines: 1,
                participants: 2,
                effect_work_units: 6,
                events: 1,
            },
            Self::FlaskNew => ItemUseUsage {
                touched_item_instances: 2,
                location_custody_lines: source_line + 1,
                transform_lines: 1,
                participants: 2,
                effect_work_units: 6,
                events: 1,
            },
        }
    }
}

fn check_state(value: &OneItemStateV1) -> Result<(), AuditError> {
    check_uuid_v7(&value.item_instance_id)?;
    check_uuid_v7(&value.world_id)?;
    check_definition(value.definition.as_ref())
}

fn check_entry(value: &OneItemContainerEntryV1) -> Result<(), AuditError> {
    check_uuid_v7(&value.parent_item_instance_id)?;
    if value.placement_ordinal == 0 {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

/// The flask part against the used stack: returns the flask shape.
fn check_flask(
    flask: &OneItemUseFlaskV1,
    source_before: &OneItemStateV1,
    source_entry: &OneItemContainerEntryV1,
) -> Result<ItemUseShape, AuditError> {
    let after = flask.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let entry = flask.entry.as_ref().ok_or(AuditError::InvalidInput)?;
    check_state(after)?;
    check_entry(entry)?;
    // BAGS-USE-1: source and flask may have different parents (nested source, main-backpack
    // flask). The writer and SQL prove the same Character's carried tree; an audit entry alone
    // cannot prove ancestry. Ordinals collide only within one parent. Self-parenting stays invalid.
    if after.world_id != source_before.world_id
        || after.lifecycle != ITEM_LIFECYCLE_LIVE
        || after.item_instance_id == source_before.item_instance_id
        || after.definition == source_before.definition
        || entry.parent_item_instance_id == after.item_instance_id
        || (entry.parent_item_instance_id == source_entry.parent_item_instance_id
            && entry.placement_ordinal == source_entry.placement_ordinal)
    {
        return Err(AuditError::InvalidInput);
    }
    match flask.before.as_ref() {
        Some(before) => {
            check_state(before)?;
            if before.item_instance_id != after.item_instance_id
                || before.world_id != after.world_id
                || before.definition != after.definition
                || before.lifecycle != ITEM_LIFECYCLE_LIVE
                || before.quantity == 0
                || before.quantity >= GAMEITEM01_STACK_QUANTITY_MAX
                || after.quantity != before.quantity + UNITS_PER_USE
            {
                return Err(AuditError::InvalidInput);
            }
            Ok(ItemUseShape::FlaskStack)
        }
        None => {
            if after.quantity != UNITS_PER_USE {
                return Err(AuditError::InvalidInput);
            }
            Ok(ItemUseShape::FlaskNew)
        }
    }
}

/// Complete closed item-use shape, unit conservation and every registered bound; returns the
/// shape.
pub fn check_item_use(value: &OneItemUseV1) -> Result<ItemUseShape, AuditError> {
    let before = value
        .source_before
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    let after = value
        .source_after
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    let entry = value
        .source_entry
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    let cause = value.cause.as_ref().ok_or(AuditError::InvalidInput)?;
    let command = cause.command_ref.as_ref().ok_or(AuditError::InvalidInput)?;
    check_state(before)?;
    check_state(after)?;
    check_entry(entry)?;
    check_content_key(&cause.content_revision)?;
    check_content_key(&cause.ruleset_revision)?;
    check_content_key(&cause.sim_revision)?;
    check_uuid_v7(&command.game_session_id)?;
    check_uuid_v7(&value.character_id)?;
    check_uuid_v7(&value.world_id)?;
    check_uuid_v7(&value.channel_id)?;
    check_uuid_v7(&value.expected_game_session_id)?;
    if !matches!(
        cause.kind,
        ITEM_USE_CAUSE_FOOD
            | ITEM_USE_CAUSE_POTION
            | super::item_use::mining::PICK_REFINE_CAUSE
            | super::item_use::mount_taming::TAMING_CAUSE
    ) || command.command_id == 0
        || command.game_session_id != value.expected_game_session_id
        || value.expected_session_generation == 0
        || value.runtime_scope_ownership_generation == 0
        || before.world_id != value.world_id
        || before.item_instance_id != after.item_instance_id
        || before.world_id != after.world_id
        || before.definition != after.definition
        || before.lifecycle != ITEM_LIFECYCLE_LIVE
        || before.quantity == 0
        || before.quantity > GAMEITEM01_STACK_QUANTITY_MAX
        || entry.parent_item_instance_id == before.item_instance_id
    {
        return Err(AuditError::InvalidInput);
    }
    // Exactly one unit leaves: the stack keeps its identity, or retires at zero.
    let source_retires = before.quantity == UNITS_PER_USE;
    let expected_after = if source_retires {
        (ITEM_LIFECYCLE_RETIRED, 0)
    } else {
        (ITEM_LIFECYCLE_LIVE, before.quantity - UNITS_PER_USE)
    };
    if (after.lifecycle, after.quantity) != expected_after {
        return Err(AuditError::InvalidInput);
    }
    let shape = match (cause.kind, value.flask.as_ref(), value.flask_missing) {
        (
            ITEM_USE_CAUSE_FOOD
            | ITEM_USE_CAUSE_POTION
            | super::item_use::mount_taming::TAMING_CAUSE,
            None,
            false,
        ) => ItemUseShape::Burn,
        (ITEM_USE_CAUSE_POTION, None, true) => ItemUseShape::BurnFlaskMissing,
        (
            ITEM_USE_CAUSE_POTION | super::item_use::mining::PICK_REFINE_CAUSE,
            Some(flask),
            false,
        ) => check_flask(flask, before, entry)?,
        _ => return Err(AuditError::InvalidInput),
    };
    shape.usage(source_retires).check()?;
    Ok(shape)
}

/// RL-07 payload gate for `item_use`: size before decode, canonical round trip, closed shape
/// and per-field bounds.
pub fn decode_item_use_payload(wire: &[u8]) -> Result<OneItemUseV1, AuditError> {
    if wire.len() > ITEM_USE_RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = OneItemTransactionV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire
        || value.interpretation_revision != mint::INTERPRETATION_REVISION
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::ItemUse(item_use)) = value.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_item_use(&item_use)?;
    Ok(item_use)
}

/// RL-07 envelope gate for `item_use`: the shared envelope gate, the payload gate, and the
/// envelope's scope, session, fence and player command equal to the payload's.
pub fn decode_item_use_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, OneItemUseV1), AuditError> {
    let value = mint::decode_common_envelope_within(wire, ITEM_USE_RL07_ENVELOPE_BYTES_MAX)?;
    let item_use = decode_item_use_payload(&value.payload)?;
    let command = item_use
        .cause
        .as_ref()
        .and_then(|cause| cause.command_ref.as_ref())
        .ok_or(AuditError::InvalidInput)?;
    let envelope_command = mint::CausationRefV1 {
        cause: Some(mint::CausationV1::Command(mint::EnvelopeCommandRefV1 {
            game_session_id: command.game_session_id.clone(),
            command_id: command.command_id,
        })),
    };
    if value.world_id.as_deref() != Some(item_use.world_id.as_slice())
        || value.channel_id.as_deref() != Some(item_use.channel_id.as_slice())
        || value.game_session_id.as_deref() != Some(command.game_session_id.as_slice())
        || value.command_id != Some(command.command_id)
        || value.connection_generation != Some(item_use.expected_session_generation)
        || value.causation.as_ref() != Some(&envelope_command)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((value, item_use))
}

/// Envelope identity fixed before the commit attempt that can become ambiguous.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemUseEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: &'a str,
}

/// Build the exact immutable event bytes of one use; re-decoded through both gates before
/// they are returned.
pub fn encode_item_use_event(
    identity: ItemUseEventIdentity<'_>,
    item_use: OneItemUseV1,
    tuple: mint::Type2EventTuple,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    mint::check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    check_item_use(&item_use)?;
    let (world_id, channel_id) = (item_use.world_id.clone(), item_use.channel_id.clone());
    let connection_generation = item_use.expected_session_generation;
    let command = item_use
        .cause
        .as_ref()
        .and_then(|cause| cause.command_ref.clone())
        .ok_or(AuditError::InvalidInput)?;
    let payload = encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::ItemUse(item_use)),
        },
        ITEM_USE_RL07_PAYLOAD_BYTES_MAX,
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
        game_session_id: Some(command.game_session_id.clone()),
        connection_generation: Some(connection_generation),
        command_id: Some(command.command_id),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: identity.transaction_id.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(mint::CausationRefV1 {
            cause: Some(mint::CausationV1::Command(mint::EnvelopeCommandRefV1 {
                game_session_id: command.game_session_id,
                command_id: command.command_id,
            })),
        }),
        server_build_id: identity.server_build_id.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..EventEnvelopeV1::default()
    };
    let wire = encode_bounded(&envelope, ITEM_USE_RL07_ENVELOPE_BYTES_MAX)?;
    let (decoded, _) = decode_item_use_envelope(&wire)?;
    if decoded != envelope {
        return Err(AuditError::InvalidInput);
    }
    Ok(wire)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::super::item_mint_audit::{OneItemTypedDefinitionRevisionV1, decode_payload};
    use super::*;
    use serde_json::Value;

    const RESOURCE_REGISTRY: &str = oteryn_protocol_oteryn::RESOURCE_LIMITS_REGISTRY_JSON;

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
                    .starts_with("apps/game-server/src/durability/item_use_audit.rs")),
            "{id} names the item-use consumer"
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

    fn fixed(tag: u8) -> [u8; 16] {
        uuid(tag).try_into().unwrap()
    }

    fn state(id: u8, key: &str, quantity: u32, lifecycle: u32) -> OneItemStateV1 {
        OneItemStateV1 {
            item_instance_id: uuid(id),
            world_id: uuid(1),
            definition: Some(OneItemTypedDefinitionRevisionV1 {
                family: "ItemType".into(),
                production_key: key.into(),
                revision_ref: "rev-1".into(),
            }),
            quantity,
            lifecycle,
            key_number: 0,
        }
    }

    fn entry(ordinal: u64) -> OneItemContainerEntryV1 {
        OneItemContainerEntryV1 {
            parent_item_instance_id: uuid(7),
            placement_ordinal: ordinal,
        }
    }

    /// Three apples eaten: the stack keeps its identity at two.
    fn burn() -> OneItemUseV1 {
        OneItemUseV1 {
            source_before: Some(state(9, "apple", 3, ITEM_LIFECYCLE_LIVE)),
            source_after: Some(state(9, "apple", 2, ITEM_LIFECYCLE_LIVE)),
            source_entry: Some(entry(4)),
            cause: Some(OneItemUseCauseV1 {
                kind: ITEM_USE_CAUSE_FOOD,
                content_revision: "content-1".into(),
                ruleset_revision: "rules-1".into(),
                sim_revision: "sim-1".into(),
                command_ref: Some(OneItemCommandRefV1 {
                    game_session_id: uuid(5),
                    command_id: 12,
                }),
            }),
            character_id: uuid(3),
            world_id: uuid(1),
            channel_id: uuid(2),
            runtime_scope_ownership_generation: 1,
            expected_session_generation: 2,
            expected_game_session_id: uuid(5),
            flask: None,
            flask_missing: false,
        }
    }

    /// The last potion drunk: the stack retires and a flask stack grows by one.
    fn flask_stack() -> OneItemUseV1 {
        let mut value = burn();
        value.source_before = Some(state(9, "mana-potion", 1, ITEM_LIFECYCLE_LIVE));
        value.source_after = Some(state(9, "mana-potion", 0, ITEM_LIFECYCLE_RETIRED));
        value.cause.as_mut().unwrap().kind = ITEM_USE_CAUSE_POTION;
        value.flask = Some(OneItemUseFlaskV1 {
            before: Some(state(10, "empty-flask", 4, ITEM_LIFECYCLE_LIVE)),
            after: Some(state(10, "empty-flask", 5, ITEM_LIFECYCLE_LIVE)),
            entry: Some(entry(2)),
        });
        value
    }

    /// A potion with several units and no stack of flasks: a fresh flask item.
    fn flask_new() -> OneItemUseV1 {
        let mut value = flask_stack();
        value.source_before = Some(state(9, "mana-potion", 5, ITEM_LIFECYCLE_LIVE));
        value.source_after = Some(state(9, "mana-potion", 4, ITEM_LIFECYCLE_LIVE));
        value.flask = Some(OneItemUseFlaskV1 {
            before: None,
            after: Some(state(11, "empty-flask", 1, ITEM_LIFECYCLE_LIVE)),
            entry: Some(entry(5)),
        });
        value
    }

    fn flask_missing() -> OneItemUseV1 {
        let mut value = flask_stack();
        value.flask = None;
        value.flask_missing = true;
        value
    }

    fn identity() -> ItemUseEventIdentity<'static> {
        ItemUseEventIdentity {
            event_id: fixed(80),
            transaction_id: fixed(81),
            occurred_at_unix_ms: 1_790_000_000_000,
            server_build_id: "oteryn-game-server-test",
        }
    }

    #[test]
    fn nested_source_and_main_flask_can_have_equal_ordinals_in_different_containers() {
        let mut value = flask_stack();
        value.source_entry.as_mut().unwrap().parent_item_instance_id = uuid(8);
        value
            .flask
            .as_mut()
            .unwrap()
            .entry
            .as_mut()
            .unwrap()
            .placement_ordinal = value.source_entry.as_ref().unwrap().placement_ordinal;
        assert!(encode_item_use_event(identity(), value, mint::Type2EventTuple::V1).is_ok());
    }

    #[test]
    fn item_use_rows_equal_the_registry() {
        assert_eq!(
            registered("DUR03-RL-01-ITEM-USE"),
            ITEM_USE_RL01_TOUCHED_ITEM_INSTANCES_MAX
        );
        assert_eq!(
            registered("DUR03-RL-02-ITEM-USE"),
            ITEM_USE_RL02_LOCATION_CUSTODY_LINES_MAX
        );
        assert_eq!(
            registered("DUR03-RL-04-ITEM-USE"),
            ITEM_USE_RL04_TRANSFORM_LINES_MAX
        );
        assert_eq!(
            registered("DUR03-RL-06-ITEM-USE-PARTICIPANTS"),
            ITEM_USE_RL06_PARTICIPANTS_MAX
        );
        assert_eq!(
            registered("DUR03-RL-06-ITEM-USE-EFFECT-WORK-UNITS"),
            ITEM_USE_RL06_EFFECT_WORK_UNITS_MAX
        );
        assert_eq!(
            registered("DUR03-RL-07-ITEM-USE-EVENTS"),
            ITEM_USE_RL07_EVENTS_MAX
        );
        // The one-item rows stay as they are.
        assert_eq!(mint::RL04_TRANSFORM_LINES_MAX, 0);
        assert_eq!(mint::RL01_TOUCHED_ITEM_INSTANCES_MAX, 2);
        assert_eq!(mint::RL06_EFFECT_WORK_UNITS_MAX, 6);
    }

    #[test]
    fn every_shape_round_trips_and_stays_within_its_rows() {
        for (value, shape) in [
            (burn(), ItemUseShape::Burn),
            (flask_stack(), ItemUseShape::FlaskStack),
            (flask_new(), ItemUseShape::FlaskNew),
            (flask_missing(), ItemUseShape::BurnFlaskMissing),
        ] {
            assert_eq!(check_item_use(&value), Ok(shape));
            for retires in [false, true] {
                assert_eq!(shape.usage(retires).check(), Ok(()), "{shape:?}");
            }
            let wire = encode_item_use_event(identity(), value.clone(), mint::Type2EventTuple::V1)
                .unwrap();
            assert!(wire.len() <= ITEM_USE_RL07_ENVELOPE_BYTES_MAX);
            let (envelope, decoded) = decode_item_use_envelope(&wire).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(envelope.connection_generation, Some(2));
            // The MINT gate refuses the item-use payload.
            assert_eq!(
                decode_payload(&envelope.payload),
                Err(AuditError::InvalidInput)
            );
        }
        // The widest shape reaches the registered maxima exactly.
        assert_eq!(ItemUseShape::FlaskNew.usage(true), ItemUseUsage::CEILING);
    }

    #[test]
    fn a_usage_above_a_row_is_capacity_exceeded() {
        let ceiling = ItemUseUsage::CEILING;
        for usage in [
            ItemUseUsage {
                touched_item_instances: ceiling.touched_item_instances + 1,
                ..ceiling
            },
            ItemUseUsage {
                location_custody_lines: ceiling.location_custody_lines + 1,
                ..ceiling
            },
            ItemUseUsage {
                transform_lines: ceiling.transform_lines + 1,
                ..ceiling
            },
            ItemUseUsage {
                participants: ceiling.participants + 1,
                ..ceiling
            },
            ItemUseUsage {
                effect_work_units: ceiling.effect_work_units + 1,
                ..ceiling
            },
            ItemUseUsage {
                events: ceiling.events + 1,
                ..ceiling
            },
        ] {
            assert_eq!(usage.check(), Err(AuditError::CapacityExceeded));
        }
    }

    #[test]
    fn the_worst_case_fits_the_one_item_payload_and_envelope_rows() {
        let long = |byte: u8, len: usize| String::from_utf8(vec![byte; len]).unwrap();
        let definition = |byte| {
            Some(OneItemTypedDefinitionRevisionV1 {
                family: long(b'f', 128),
                production_key: long(byte, 512),
                revision_ref: long(byte, 512),
            })
        };
        let mut value = flask_new();
        let source = value.source_before.as_mut().unwrap();
        source.definition = definition(b'a');
        source.quantity = GAMEITEM01_STACK_QUANTITY_MAX;
        let source = value.source_after.as_mut().unwrap();
        source.definition = definition(b'a');
        source.quantity = GAMEITEM01_STACK_QUANTITY_MAX - 1;
        value
            .flask
            .as_mut()
            .unwrap()
            .after
            .as_mut()
            .unwrap()
            .definition = definition(b'b');
        value.entry_ordinals_max();
        let cause = value.cause.as_mut().unwrap();
        cause.content_revision = long(b'c', 512);
        cause.ruleset_revision = long(b'r', 512);
        cause.sim_revision = long(b's', 512);
        cause.command_ref.as_mut().unwrap().command_id = u64::MAX;
        value.runtime_scope_ownership_generation = u64::MAX;
        value.expected_session_generation = u64::MAX;
        let mut identity = identity();
        identity.server_build_id = "b".repeat(128).leak();
        identity.occurred_at_unix_ms = i64::MAX;
        let wire = encode_item_use_event(identity, value, mint::Type2EventTuple::V1).unwrap();
        assert!(
            wire.len() <= ITEM_USE_RL07_ENVELOPE_BYTES_MAX,
            "{}",
            wire.len()
        );
        let (envelope, _) = decode_item_use_envelope(&wire).unwrap();
        assert!(envelope.payload.len() <= ITEM_USE_RL07_PAYLOAD_BYTES_MAX);
        // Registered by DUR03-RL-07-ITEM-USE-PAYLOAD-BYTES / -ENVELOPE-BYTES.
        assert_eq!(
            (envelope.payload.len() as u64, wire.len() as u64),
            (
                registered("DUR03-RL-07-ITEM-USE-PAYLOAD-BYTES"),
                registered("DUR03-RL-07-ITEM-USE-ENVELOPE-BYTES")
            )
        );
    }

    impl OneItemUseV1 {
        fn entry_ordinals_max(&mut self) {
            self.source_entry.as_mut().unwrap().placement_ordinal = u64::MAX;
            let flask = self.flask.as_mut().unwrap();
            flask.entry.as_mut().unwrap().placement_ordinal = u64::MAX - 1;
        }
    }

    #[test]
    fn mount_taming_is_an_audited_one_unit_burn_without_a_flask() {
        let mut value = burn();
        value.cause.as_mut().unwrap().kind = super::super::item_use::mount_taming::TAMING_CAUSE;
        assert_eq!(check_item_use(&value), Ok(ItemUseShape::Burn));
        value.flask = flask_new().flask;
        assert_eq!(check_item_use(&value), Err(AuditError::InvalidInput));
        value.flask = None;
        value.source_after.as_mut().unwrap().quantity =
            value.source_before.as_ref().unwrap().quantity;
        assert_eq!(check_item_use(&value), Err(AuditError::InvalidInput));
    }

    #[test]
    fn every_broken_invariant_is_rejected() {
        let mut cases: Vec<(&str, OneItemUseV1)> = Vec::new();
        let mut value = burn();
        value.source_after.as_mut().unwrap().quantity = 1;
        cases.push(("two units leaving", value));
        let mut value = burn();
        value.source_after.as_mut().unwrap().quantity = 3;
        cases.push(("no unit leaving", value));
        let mut value = flask_stack();
        value.source_after.as_mut().unwrap().lifecycle = ITEM_LIFECYCLE_LIVE;
        cases.push(("the last unit leaving a live stack", value));
        let mut value = burn();
        value.source_after.as_mut().unwrap().item_instance_id = uuid(8);
        cases.push(("another identity after", value));
        let mut value = burn();
        value.cause.as_mut().unwrap().kind = 3;
        cases.push(("an open cause", value));
        let mut value = burn();
        value.cause.as_mut().unwrap().kind = 0;
        cases.push(("no cause", value));
        let mut value = burn();
        value
            .cause
            .as_mut()
            .unwrap()
            .command_ref
            .as_mut()
            .unwrap()
            .command_id = 0;
        cases.push(("no command", value));
        let mut value = burn();
        value.expected_game_session_id = uuid(6);
        cases.push(("another session than the command's", value));
        let mut value = burn();
        value.expected_session_generation = 0;
        cases.push(("no fence", value));
        let mut value = burn();
        value.runtime_scope_ownership_generation = 0;
        cases.push(("no scope generation", value));
        let mut value = burn();
        value.world_id = uuid(2);
        cases.push(("another World", value));
        let mut value = burn();
        value.source_entry.as_mut().unwrap().placement_ordinal = 0;
        cases.push(("entry ordinal 0", value));
        let mut value = burn();
        value.source_before.as_mut().unwrap().quantity = GAMEITEM01_STACK_QUANTITY_MAX + 1;
        cases.push(("a stack above the ceiling", value));
        let mut value = burn();
        value.flask = flask_new().flask;
        cases.push(("food with a flask", value));
        let mut value = burn();
        value.flask_missing = true;
        cases.push(("food with a missing flask", value));
        let mut value = flask_stack();
        value.flask_missing = true;
        cases.push(("a flask and a missing flask", value));
        let mut value = flask_stack();
        let ordinal = value.source_entry.as_ref().unwrap().placement_ordinal;
        value
            .flask
            .as_mut()
            .unwrap()
            .entry
            .as_mut()
            .unwrap()
            .placement_ordinal = ordinal;
        cases.push(("a flask entry at the source ordinal", value));
        let mut value = flask_stack();
        value
            .flask
            .as_mut()
            .unwrap()
            .after
            .as_mut()
            .unwrap()
            .quantity = 6;
        cases.push(("two flasks added", value));
        let mut value = flask_stack();
        value
            .flask
            .as_mut()
            .unwrap()
            .before
            .as_mut()
            .unwrap()
            .quantity = GAMEITEM01_STACK_QUANTITY_MAX;
        cases.push(("a full flask stack growing", value));
        let mut value = flask_stack();
        value
            .flask
            .as_mut()
            .unwrap()
            .after
            .as_mut()
            .unwrap()
            .item_instance_id = uuid(12);
        cases.push(("a flask stack changing identity", value));
        let mut value = flask_stack();
        value
            .flask
            .as_mut()
            .unwrap()
            .entry
            .as_mut()
            .unwrap()
            .parent_item_instance_id = value
            .flask
            .as_ref()
            .unwrap()
            .after
            .as_ref()
            .unwrap()
            .item_instance_id
            .clone();
        cases.push(("a flask inside itself", value));
        let mut value = flask_new();
        value
            .flask
            .as_mut()
            .unwrap()
            .after
            .as_mut()
            .unwrap()
            .quantity = 2;
        cases.push(("a fresh flask of two units", value));
        let mut value = flask_new();
        value
            .flask
            .as_mut()
            .unwrap()
            .after
            .as_mut()
            .unwrap()
            .item_instance_id = uuid(9);
        cases.push(("the flask being the potion", value));
        let mut value = flask_new();
        value.flask.as_mut().unwrap().after =
            Some(state(11, "mana-potion", 1, ITEM_LIFECYCLE_LIVE));
        cases.push(("the flask of the potion's definition", value));
        let mut value = flask_new();
        value.flask.as_mut().unwrap().entry = None;
        cases.push(("a flask without an entry", value));
        for (name, value) in cases {
            assert_eq!(
                check_item_use(&value),
                Err(AuditError::InvalidInput),
                "{name}"
            );
            assert!(
                encode_item_use_event(identity(), value, mint::Type2EventTuple::V1).is_err(),
                "{name}"
            );
        }
    }

    #[test]
    fn the_envelope_must_carry_the_command_and_the_fence() {
        let wire =
            encode_item_use_event(identity(), flask_new(), mint::Type2EventTuple::V1).unwrap();
        let envelope = mint::decode_common_envelope(&wire).unwrap();
        for tamper in 0..4 {
            let mut other = envelope.clone();
            match tamper {
                0 => other.command_id = Some(13),
                1 => other.connection_generation = Some(3),
                2 => other.causation = None,
                _ => other.channel_id = Some(uuid(9)),
            }
            let bytes = other.encode_to_vec();
            assert_eq!(
                decode_item_use_envelope(&bytes).map(|_| ()),
                Err(AuditError::InvalidInput),
                "tamper {tamper}"
            );
        }
    }

    #[test]
    fn oversize_and_non_canonical_payloads_are_rejected_before_use() {
        let wire = vec![0; ITEM_USE_RL07_PAYLOAD_BYTES_MAX + 1];
        assert_eq!(
            decode_item_use_payload(&wire),
            Err(AuditError::CapacityExceeded)
        );
        let wire = vec![0; ITEM_USE_RL07_ENVELOPE_BYTES_MAX + 1];
        assert_eq!(
            decode_item_use_envelope(&wire).map(|_| ()),
            Err(AuditError::CapacityExceeded)
        );
        let mut payload = OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::ItemUse(burn())),
        }
        .encode_to_vec();
        // An unknown field fails the canonical round trip.
        payload.extend_from_slice(&[0xf8, 0x01, 0x01]);
        assert_eq!(
            decode_item_use_payload(&payload),
            Err(AuditError::InvalidInput)
        );
    }
}
