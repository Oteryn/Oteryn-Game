//! DUR-03 shapes of the timed-item writes and the expiry durable-audit encoding (TIMED-RT-1b;
//! decision `TIMEDITEM0B-RUNTIME-CHARGES-AND-DURATION-V1` §8 and §12, DUR-03 §39.3 timed-item
//! amendment): event type 2 (`oteryn.events.v1.OneItemTransactionV1`, operation
//! `timed_expiry`, oneof tag 7) in the normative ANL-01 `EventEnvelope`.
//!
//! §12 admits five one-item shapes, each with its own `DUR03-RL-*-TIMED*` rows: the checkpoint
//! and the composed checkpoint (no location, no transform), the expiry transform (one
//! `PRESERVE_INSTANCE` TRANSFORM, 1 input / 1 output, in place), the expiry burn (the item to
//! `RETIRED`, one location line) and the composed expiry burn. A composed shape adds the build
//! receipt's own rows, which are charged against that receipt's registry rows, not these.
//!
//! One event per expiry: the item, the definition before and after, its holder location, the
//! row before and after (charges, remaining time, revision), the reason and the TransactionId
//! (the envelope's). Every bound is checked before encode allocation and oversize input is
//! rejected, never truncated.

use super::item_mint_audit::{
    self as mint, AuditError, EventEnvelopeV1, ITEM_LIFECYCLE_LIVE, OneItemOperationV1,
    OneItemStateV1, OneItemTransactionV1, RL07_ENVELOPE_BYTES_MAX, RL07_PAYLOAD_BYTES_MAX,
    TransactionEventRefV1, check_definition, check_uuid_v7, encode_bounded,
};
use super::item_transfer_audit::{ITEM_LIFECYCLE_RETIRED, OneItemContainerEntryV1};
use crate::domain::timed_item::{TIMEDITEM0_RL_01_CHARGES_MAX, TIMEDITEM0_RL_02_REMAINING_MS_MAX};
use prost::Message;
use sha2::{Digest, Sha256};

// Timed-shape rows (decision §12; RESOURCE_LIMITS_REGISTRY.json `DUR03-RL-*-TIMED*`).
pub const TIMED_RL01_TOUCHED_ITEM_INSTANCES_MAX: u64 = 1;
/// Checkpoint, composed checkpoint and expiry transform: no location line.
pub const TIMED_RL02_WRITE_LOCATION_LINES_MAX: u64 = 0;
/// Expiry burn and composed expiry burn: the one removed slot or entry.
pub const TIMED_RL02_EXPIRY_BURN_LOCATION_LINES_MAX: u64 = 1;
/// `DUR03-RL-04-TIMED-EXPIRY`: transform inputs, and outputs, of the expiry transform.
pub const TIMED_RL04_EXPIRY_TRANSFORM_LINES_MAX: u64 = 1;
pub const TIMED_RL06_CHECKPOINT_PARTICIPANTS_MAX: u64 = 1;
pub const TIMED_RL06_CHECKPOINT_EFFECT_WORK_UNITS_MAX: u64 = 2;
pub const TIMED_RL06_EXPIRY_PARTICIPANTS_MAX: u64 = 1;
pub const TIMED_RL06_EXPIRY_EFFECT_WORK_UNITS_MAX: u64 = 4;
pub const TIMED_RL07_EVENTS_MAX: u64 = 1;
pub const TIMED_RL08_RETRY_WORK_UNITS_MAX: u64 = 3;

/// `reason` of the event: `Expire {reason}` (§8).
pub const TIMED_EXPIRY_REASON_TIME_EXHAUSTED: u32 = 1;
pub const TIMED_EXPIRY_REASON_CHARGES_EXHAUSTED: u32 = 2;

/// The §12 one-item shapes RT-1b admits. The use form and put out are RT-1c's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedShape {
    Checkpoint,
    /// The checkpoint plus the build receipt in one transaction (EXERCISE-0 §5.3).
    ComposedCheckpoint,
    ExpiryTransform,
    ExpiryBurn,
    /// The expiry burn plus the last build receipt in one transaction (EXERCISE-0 §5.3).
    ComposedExpiryBurn,
}

/// Per-transaction counts of a timed shape, its own receipt's rows excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedShapeUsage {
    pub touched_item_instances: u64,
    pub location_lines: u64,
    pub transform_inputs: u64,
    pub transform_outputs: u64,
    pub participants: u64,
    pub effect_work_units: u64,
    pub events: u64,
    pub retry_work_units: u64,
}

impl TimedShape {
    /// The shape's own usage: the participant and its row write (a checkpoint), plus the
    /// TRANSFORM and the record (an expiry transform) or the removal and the retirement (an
    /// expiry burn). A checkpoint has no event; an expiry has its one event.
    #[must_use]
    pub const fn usage(self) -> TimedShapeUsage {
        match self {
            Self::Checkpoint | Self::ComposedCheckpoint => TimedShapeUsage {
                touched_item_instances: 1,
                location_lines: 0,
                transform_inputs: 0,
                transform_outputs: 0,
                participants: 1,
                effect_work_units: 2,
                events: 0,
                retry_work_units: TIMED_RL08_RETRY_WORK_UNITS_MAX,
            },
            Self::ExpiryTransform => TimedShapeUsage {
                touched_item_instances: 1,
                location_lines: 0,
                transform_inputs: 1,
                transform_outputs: 1,
                participants: 1,
                effect_work_units: 4,
                events: 1,
                retry_work_units: TIMED_RL08_RETRY_WORK_UNITS_MAX,
            },
            Self::ExpiryBurn | Self::ComposedExpiryBurn => TimedShapeUsage {
                touched_item_instances: 1,
                location_lines: 1,
                transform_inputs: 0,
                transform_outputs: 0,
                participants: 1,
                effect_work_units: 4,
                events: 1,
                retry_work_units: TIMED_RL08_RETRY_WORK_UNITS_MAX,
            },
        }
    }

    /// The registered ceilings of the shape (§12).
    #[must_use]
    pub const fn ceiling(self) -> TimedShapeUsage {
        let (location_lines, transform_lines, participants, effect_work_units) = match self {
            Self::Checkpoint | Self::ComposedCheckpoint => (
                TIMED_RL02_WRITE_LOCATION_LINES_MAX,
                0,
                TIMED_RL06_CHECKPOINT_PARTICIPANTS_MAX,
                TIMED_RL06_CHECKPOINT_EFFECT_WORK_UNITS_MAX,
            ),
            Self::ExpiryTransform => (
                TIMED_RL02_WRITE_LOCATION_LINES_MAX,
                TIMED_RL04_EXPIRY_TRANSFORM_LINES_MAX,
                TIMED_RL06_EXPIRY_PARTICIPANTS_MAX,
                TIMED_RL06_EXPIRY_EFFECT_WORK_UNITS_MAX,
            ),
            Self::ExpiryBurn | Self::ComposedExpiryBurn => (
                TIMED_RL02_EXPIRY_BURN_LOCATION_LINES_MAX,
                0,
                TIMED_RL06_EXPIRY_PARTICIPANTS_MAX,
                TIMED_RL06_EXPIRY_EFFECT_WORK_UNITS_MAX,
            ),
        };
        TimedShapeUsage {
            touched_item_instances: TIMED_RL01_TOUCHED_ITEM_INSTANCES_MAX,
            location_lines,
            transform_inputs: transform_lines,
            transform_outputs: transform_lines,
            participants,
            effect_work_units,
            events: TIMED_RL07_EVENTS_MAX,
            retry_work_units: TIMED_RL08_RETRY_WORK_UNITS_MAX,
        }
    }

    /// `usage` against the shape's rows, before any write. A transform in a shape without one
    /// is not admitted (`INVALID_INPUT`, as `DUR03-RL-04`); every other excess is
    /// `CAPACITY_EXCEEDED`.
    pub fn check(self, usage: &TimedShapeUsage) -> Result<(), AuditError> {
        let ceiling = self.ceiling();
        if ceiling.transform_inputs == 0
            && (usage.transform_inputs > 0 || usage.transform_outputs > 0)
        {
            return Err(AuditError::InvalidInput);
        }
        if usage.touched_item_instances > ceiling.touched_item_instances
            || usage.location_lines > ceiling.location_lines
            || usage.transform_inputs > ceiling.transform_inputs
            || usage.transform_outputs > ceiling.transform_outputs
            || usage.participants > ceiling.participants
            || usage.effect_work_units > ceiling.effect_work_units
            || usage.events > ceiling.events
            || usage.retry_work_units > ceiling.retry_work_units
        {
            return Err(AuditError::CapacityExceeded);
        }
        Ok(())
    }
}

/// One side of the timed row in the event: absent values are a spent row (after a transform
/// to a target that is not timed); revision 0 is an item without a row (its full values).
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemTimedRowV1 {
    #[prost(uint32, optional, tag = "1")]
    pub charges: Option<u32>,
    #[prost(uint64, optional, tag = "2")]
    pub remaining_ms: Option<u64>,
    #[prost(uint64, tag = "3")]
    pub revision: u64,
}

/// One expiry (§8): an expiry transform (the definition changes in place, the item stays
/// live) or an expiry burn (the item retires from its container entry).
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemTimedExpiryV1 {
    #[prost(message, optional, tag = "1")]
    pub before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub after: Option<OneItemStateV1>,
    /// The holder.
    #[prost(bytes = "vec", tag = "3")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub channel_id: Vec<u8>,
    #[prost(uint64, tag = "6")]
    pub runtime_scope_ownership_generation: u64,
    /// The holder location before: exactly one of a container entry and the equipment
    /// `container` slot. A burn's location is always an entry.
    #[prost(message, optional, tag = "7")]
    pub container_entry: Option<OneItemContainerEntryV1>,
    #[prost(bool, tag = "8")]
    pub equipment_container_slot: bool,
    #[prost(uint32, tag = "9")]
    pub reason: u32,
    #[prost(message, optional, tag = "10")]
    pub row_before: Option<OneItemTimedRowV1>,
    #[prost(message, optional, tag = "11")]
    pub row_after: Option<OneItemTimedRowV1>,
}

fn check_state(value: &OneItemStateV1) -> Result<(), AuditError> {
    check_uuid_v7(&value.item_instance_id)?;
    check_uuid_v7(&value.world_id)?;
    check_definition(value.definition.as_ref())
}

/// Stored values: charges 1..=`TIMEDITEM0-RL-01`, time 0..=`TIMEDITEM0-RL-02`.
fn check_row_values(row: &OneItemTimedRowV1) -> Result<(), AuditError> {
    if row
        .charges
        .is_some_and(|value| !(1..=TIMEDITEM0_RL_01_CHARGES_MAX).contains(&value))
        || row
            .remaining_ms
            .is_some_and(|value| value > TIMEDITEM0_RL_02_REMAINING_MS_MAX)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

/// Complete closed expiry shape and every registered bound; returns the shape.
pub fn check_timed_expiry(value: &OneItemTimedExpiryV1) -> Result<TimedShape, AuditError> {
    let before = value.before.as_ref().ok_or(AuditError::InvalidInput)?;
    let after = value.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let row_before = value.row_before.as_ref().ok_or(AuditError::InvalidInput)?;
    let row_after = value.row_after.as_ref().ok_or(AuditError::InvalidInput)?;
    check_state(before)?;
    check_state(after)?;
    check_uuid_v7(&value.character_id)?;
    check_uuid_v7(&value.world_id)?;
    check_uuid_v7(&value.channel_id)?;
    check_row_values(row_before)?;
    check_row_values(row_after)?;
    match (
        value.container_entry.as_ref(),
        value.equipment_container_slot,
    ) {
        (Some(entry), false) => {
            check_uuid_v7(&entry.parent_item_instance_id)?;
            if entry.placement_ordinal == 0
                || entry.parent_item_instance_id == before.item_instance_id
            {
                return Err(AuditError::InvalidInput);
            }
        }
        (None, true) => {}
        _ => return Err(AuditError::InvalidInput),
    }
    let reason_value = match value.reason {
        TIMED_EXPIRY_REASON_TIME_EXHAUSTED => row_before.remaining_ms,
        TIMED_EXPIRY_REASON_CHARGES_EXHAUSTED => row_before.charges.map(u64::from),
        _ => return Err(AuditError::InvalidInput),
    };
    if before.item_instance_id != after.item_instance_id
        || before.world_id != after.world_id
        || before.world_id != value.world_id
        || before.lifecycle != ITEM_LIFECYCLE_LIVE
        || before.quantity == 0
        || value.runtime_scope_ownership_generation == 0
        // The expired value is one the item had (a spent row never expires).
        || reason_value.is_none()
        || row_before.revision == u64::MAX
    {
        return Err(AuditError::InvalidInput);
    }
    let shape = if before.definition == after.definition {
        // Burn: retired from its entry; the row stays inert and the record carries the
        // before values.
        if after.lifecycle != ITEM_LIFECYCLE_RETIRED
            || after.quantity != 0
            || value.container_entry.is_none()
            || row_after != row_before
        {
            return Err(AuditError::InvalidInput);
        }
        TimedShape::ExpiryBurn
    } else {
        // Transform in place: the same live item at the target, the row reset to the
        // target's full values (never 0 ms) or spent, one revision up.
        if after.lifecycle != ITEM_LIFECYCLE_LIVE
            || after.quantity != before.quantity
            || row_after.revision != row_before.revision + 1
            || row_after.remaining_ms == Some(0)
        {
            return Err(AuditError::InvalidInput);
        }
        TimedShape::ExpiryTransform
    };
    shape.check(&shape.usage())?;
    Ok(shape)
}

/// RL-07 payload gate for `timed_expiry`: size before decode, canonical round trip, closed
/// shape and per-field bounds.
pub fn decode_timed_expiry_payload(wire: &[u8]) -> Result<OneItemTimedExpiryV1, AuditError> {
    if wire.len() > RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = OneItemTransactionV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire
        || value.interpretation_revision != mint::INTERPRETATION_REVISION
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::TimedExpiry(expiry)) = value.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_timed_expiry(&expiry)?;
    Ok(expiry)
}

/// RL-07 envelope gate for `timed_expiry`: the shared envelope gate, the payload gate, the
/// envelope scope equal to the holder's runtime scope and no session, command or causation.
pub fn decode_timed_expiry_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, OneItemTimedExpiryV1), AuditError> {
    let value = mint::decode_common_envelope(wire)?;
    let expiry = decode_timed_expiry_payload(&value.payload)?;
    if value.world_id.as_deref() != Some(expiry.world_id.as_slice())
        || value.channel_id.as_deref() != Some(expiry.channel_id.as_slice())
        || value.game_session_id.is_some()
        || value.command_id.is_some()
        || value.connection_generation.is_some()
        || value.causation.is_some()
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((value, expiry))
}

/// Envelope identity fixed before the commit attempt that can become ambiguous.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedExpiryEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: &'a str,
}

/// Build the exact immutable event bytes of one expiry; re-decoded through both gates before
/// they are returned.
pub fn encode_timed_expiry_event(
    identity: TimedExpiryEventIdentity<'_>,
    expiry: OneItemTimedExpiryV1,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    mint::check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    check_timed_expiry(&expiry)?;
    let (world_id, channel_id) = (expiry.world_id.clone(), expiry.channel_id.clone());
    let payload = encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::TimedExpiry(expiry)),
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
    let (decoded, _) = decode_timed_expiry_envelope(&wire)?;
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

    const RESOURCE_REGISTRY: &str =
        include_str!("../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");

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
                    .starts_with("apps/game-server/src/durability/item_timed_state_audit.rs")),
            "{id} names the timed runtime consumer"
        );
        rows[0]["hard_maximum"].as_u64().unwrap()
    }

    const ALL: [TimedShape; 5] = [
        TimedShape::Checkpoint,
        TimedShape::ComposedCheckpoint,
        TimedShape::ExpiryTransform,
        TimedShape::ExpiryBurn,
        TimedShape::ComposedExpiryBurn,
    ];

    #[test]
    fn timed_rows_equal_the_registry() {
        assert_eq!(
            registered("DUR03-RL-01-TIMED"),
            TIMED_RL01_TOUCHED_ITEM_INSTANCES_MAX
        );
        assert_eq!(
            registered("DUR03-RL-02-TIMED-WRITE"),
            TIMED_RL02_WRITE_LOCATION_LINES_MAX
        );
        assert_eq!(
            registered("DUR03-RL-02-TIMED-EXPIRY-BURN"),
            TIMED_RL02_EXPIRY_BURN_LOCATION_LINES_MAX
        );
        assert_eq!(
            registered("DUR03-RL-04-TIMED-EXPIRY"),
            TIMED_RL04_EXPIRY_TRANSFORM_LINES_MAX
        );
        assert_eq!(
            registered("DUR03-RL-06-TIMED-CHECKPOINT-PARTICIPANTS"),
            TIMED_RL06_CHECKPOINT_PARTICIPANTS_MAX
        );
        assert_eq!(
            registered("DUR03-RL-06-TIMED-CHECKPOINT-EFFECT-WORK-UNITS"),
            TIMED_RL06_CHECKPOINT_EFFECT_WORK_UNITS_MAX
        );
        assert_eq!(
            registered("DUR03-RL-06-TIMED-EXPIRY-PARTICIPANTS"),
            TIMED_RL06_EXPIRY_PARTICIPANTS_MAX
        );
        assert_eq!(
            registered("DUR03-RL-06-TIMED-EXPIRY-EFFECT-WORK-UNITS"),
            TIMED_RL06_EXPIRY_EFFECT_WORK_UNITS_MAX
        );
        assert_eq!(
            registered("DUR03-RL-07-TIMED-EVENTS"),
            TIMED_RL07_EVENTS_MAX
        );
        assert_eq!(
            registered("DUR03-RL-08-TIMED"),
            TIMED_RL08_RETRY_WORK_UNITS_MAX
        );
        // The one-item rows stay as they are.
        assert_eq!(mint::RL04_TRANSFORM_LINES_MAX, 0);
        assert_eq!(mint::RL01_TOUCHED_ITEM_INSTANCES_MAX, 2);
    }

    #[test]
    fn every_shape_admits_its_own_usage_and_its_maxima() {
        for shape in ALL {
            assert_eq!(shape.check(&shape.usage()), Ok(()), "{shape:?}");
            assert_eq!(shape.check(&shape.ceiling()), Ok(()), "{shape:?}");
        }
        // §12's table.
        let rows = |shape: TimedShape| {
            let usage = shape.usage();
            (
                usage.location_lines,
                usage.transform_inputs,
                usage.transform_outputs,
                usage.participants,
                usage.effect_work_units,
            )
        };
        assert_eq!(rows(TimedShape::Checkpoint), (0, 0, 0, 1, 2));
        assert_eq!(rows(TimedShape::ComposedCheckpoint), (0, 0, 0, 1, 2));
        assert_eq!(rows(TimedShape::ExpiryTransform), (0, 1, 1, 1, 4));
        assert_eq!(rows(TimedShape::ExpiryBurn), (1, 0, 0, 1, 4));
        assert_eq!(rows(TimedShape::ComposedExpiryBurn), (1, 0, 0, 1, 4));
    }

    #[test]
    fn every_shape_rejects_max_plus_one_on_each_row() {
        type Bump = fn(&mut TimedShapeUsage);
        let bumps: [(&str, Bump); 7] = [
            ("RL-01", |usage| usage.touched_item_instances += 1),
            ("RL-02", |usage| usage.location_lines += 1),
            ("RL-06 participants", |usage| usage.participants += 1),
            ("RL-06 work units", |usage| usage.effect_work_units += 1),
            ("RL-07 events", |usage| usage.events += 1),
            ("RL-08", |usage| usage.retry_work_units += 1),
            ("RL-04 inputs", |usage| usage.transform_inputs += 1),
        ];
        for shape in ALL {
            for (row, bump) in bumps {
                let mut usage = shape.ceiling();
                bump(&mut usage);
                let expected = if row == "RL-04 inputs" && shape != TimedShape::ExpiryTransform {
                    AuditError::InvalidInput
                } else {
                    AuditError::CapacityExceeded
                };
                assert_eq!(shape.check(&usage), Err(expected), "{shape:?} {row}");
            }
        }
        // `DUR03-RL-04-TIMED-EXPIRY`: 1 / 1 passes, 2 / 1 and 1 / 2 are refused.
        let shape = TimedShape::ExpiryTransform;
        let mut usage = shape.usage();
        usage.transform_outputs = 2;
        assert_eq!(shape.check(&usage), Err(AuditError::CapacityExceeded));
        // A checkpoint has no transform at all.
        let mut usage = TimedShape::Checkpoint.usage();
        usage.transform_outputs = 1;
        assert_eq!(
            TimedShape::Checkpoint.check(&usage),
            Err(AuditError::InvalidInput)
        );
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

    fn state(key: &str, quantity: u32, lifecycle: u32) -> OneItemStateV1 {
        OneItemStateV1 {
            item_instance_id: uuid(9),
            world_id: uuid(1),
            definition: Some(OneItemTypedDefinitionRevisionV1 {
                family: "ItemType".into(),
                production_key: key.into(),
                revision_ref: "rev-1".into(),
            }),
            quantity,
            lifecycle,
        }
    }

    fn row(charges: Option<u32>, remaining_ms: Option<u64>, revision: u64) -> OneItemTimedRowV1 {
        OneItemTimedRowV1 {
            charges,
            remaining_ms,
            revision,
        }
    }

    /// Soft boots become worn soft boots: a spent row at revision + 1.
    fn transform() -> OneItemTimedExpiryV1 {
        OneItemTimedExpiryV1 {
            before: Some(state("soft-boots", 1, ITEM_LIFECYCLE_LIVE)),
            after: Some(state("worn-soft-boots", 1, ITEM_LIFECYCLE_LIVE)),
            character_id: uuid(3),
            world_id: uuid(1),
            channel_id: uuid(2),
            runtime_scope_ownership_generation: 1,
            container_entry: None,
            equipment_container_slot: true,
            reason: TIMED_EXPIRY_REASON_TIME_EXHAUSTED,
            row_before: Some(row(None, Some(60_000), 4)),
            row_after: Some(row(None, None, 5)),
        }
    }

    /// A ring without a decay target is burned from its entry; the row stays inert.
    fn burn() -> OneItemTimedExpiryV1 {
        OneItemTimedExpiryV1 {
            before: Some(state("ring", 1, ITEM_LIFECYCLE_LIVE)),
            after: Some(state("ring", 0, ITEM_LIFECYCLE_RETIRED)),
            container_entry: Some(OneItemContainerEntryV1 {
                parent_item_instance_id: uuid(7),
                placement_ordinal: 3,
            }),
            equipment_container_slot: false,
            reason: TIMED_EXPIRY_REASON_CHARGES_EXHAUSTED,
            row_before: Some(row(Some(5), None, 0)),
            row_after: Some(row(Some(5), None, 0)),
            ..transform()
        }
    }

    fn identity() -> TimedExpiryEventIdentity<'static> {
        TimedExpiryEventIdentity {
            event_id: fixed(80),
            transaction_id: fixed(81),
            occurred_at_unix_ms: 1_790_000_000_000,
            server_build_id: "oteryn-game-server-test",
        }
    }

    #[test]
    fn a_transform_and_a_burn_round_trip_within_the_one_item_ceilings() {
        for (value, shape) in [
            (transform(), TimedShape::ExpiryTransform),
            (burn(), TimedShape::ExpiryBurn),
        ] {
            assert_eq!(check_timed_expiry(&value), Ok(shape));
            let wire = encode_timed_expiry_event(identity(), value.clone()).unwrap();
            assert!(wire.len() <= RL07_ENVELOPE_BYTES_MAX);
            let (envelope, decoded) = decode_timed_expiry_envelope(&wire).unwrap();
            assert_eq!(decoded, value);
            // The MINT gate refuses the expiry payload.
            assert_eq!(
                decode_payload(&envelope.payload),
                Err(AuditError::InvalidInput)
            );
        }
    }

    #[test]
    fn the_worst_case_fits_the_one_item_payload_and_envelope_rows() {
        let long = |byte: u8, len: usize| String::from_utf8(vec![byte; len]).unwrap();
        let definition = |byte| OneItemTypedDefinitionRevisionV1 {
            family: long(b'f', 128),
            production_key: long(byte, 512),
            revision_ref: long(byte, 512),
        };
        let mut value = transform();
        value.before.as_mut().unwrap().definition = Some(definition(b'a'));
        value.before.as_mut().unwrap().quantity = u32::MAX;
        value.after.as_mut().unwrap().definition = Some(definition(b'b'));
        value.after.as_mut().unwrap().quantity = u32::MAX;
        value.runtime_scope_ownership_generation = u64::MAX;
        value.row_before = Some(row(
            Some(TIMEDITEM0_RL_01_CHARGES_MAX),
            Some(TIMEDITEM0_RL_02_REMAINING_MS_MAX),
            u64::MAX - 1,
        ));
        value.row_after = Some(row(
            Some(TIMEDITEM0_RL_01_CHARGES_MAX),
            Some(TIMEDITEM0_RL_02_REMAINING_MS_MAX),
            u64::MAX,
        ));
        let mut identity = identity();
        identity.server_build_id = "b".repeat(128).leak();
        identity.occurred_at_unix_ms = i64::MAX;
        let wire = encode_timed_expiry_event(identity, value).unwrap();
        assert!(wire.len() <= RL07_ENVELOPE_BYTES_MAX, "{}", wire.len());
    }

    #[test]
    fn every_broken_invariant_is_rejected() {
        let mut cases: Vec<(&str, OneItemTimedExpiryV1)> = Vec::new();
        let mut value = transform();
        value.row_after = Some(row(None, Some(0), 5));
        cases.push(("a transform storing 0 ms", value));
        let mut value = transform();
        value.row_after = Some(row(None, None, 4));
        cases.push(("a transform without revision + 1", value));
        let mut value = transform();
        value.after.as_mut().unwrap().quantity = 2;
        cases.push(("a transform changing the quantity", value));
        let mut value = transform();
        value.reason = TIMED_EXPIRY_REASON_CHARGES_EXHAUSTED;
        cases.push(("charges exhausted on an item without charges", value));
        let mut value = transform();
        value.reason = 3;
        cases.push(("a deadline reason (RT-1c)", value));
        let mut value = transform();
        value.row_before = Some(row(None, None, 4));
        cases.push(("a spent row expiring", value));
        let mut value = transform();
        value.container_entry = burn().container_entry;
        cases.push(("two locations", value));
        let mut value = transform();
        value.equipment_container_slot = false;
        cases.push(("no location", value));
        let mut value = burn();
        value.container_entry = None;
        value.equipment_container_slot = true;
        cases.push(("a burn from the container slot", value));
        let mut value = burn();
        value.row_after = Some(row(Some(5), None, 1));
        cases.push(("a burn writing the row", value));
        let mut value = burn();
        value.after.as_mut().unwrap().lifecycle = ITEM_LIFECYCLE_LIVE;
        cases.push(("a burn leaving the item live", value));
        let mut value = burn();
        value.row_before = Some(row(Some(0), None, 0));
        value.row_after = value.row_before.clone();
        cases.push(("0 charges stored", value));
        let mut value = burn();
        value.runtime_scope_ownership_generation = 0;
        cases.push(("no scope generation", value));
        let mut value = burn();
        value.world_id = uuid(2);
        cases.push(("another World", value));
        for (name, value) in cases {
            assert_eq!(
                check_timed_expiry(&value),
                Err(AuditError::InvalidInput),
                "{name}"
            );
            assert!(
                encode_timed_expiry_event(identity(), value).is_err(),
                "{name}"
            );
        }
    }

    #[test]
    fn oversize_and_non_canonical_payloads_are_rejected_before_use() {
        let wire = vec![0; RL07_PAYLOAD_BYTES_MAX + 1];
        assert_eq!(
            decode_timed_expiry_payload(&wire),
            Err(AuditError::CapacityExceeded)
        );
        let mut payload = OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::TimedExpiry(burn())),
        }
        .encode_to_vec();
        // An unknown field fails the canonical round trip.
        payload.extend_from_slice(&[0xf8, 0x01, 0x01]);
        assert_eq!(
            decode_timed_expiry_payload(&payload),
            Err(AuditError::InvalidInput)
        );
    }
}
