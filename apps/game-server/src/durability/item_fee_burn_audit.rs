//! Registered DUR-03 fee BURN durable-audit encoding (GOLD-FEE-1a; decision
//! `CHARACTER-GOLD-FEE-BOUNDARY-V1` §4.5, D174-D178; DUR-03 §39.3 gold fee amendment): event
//! type 2 (`oteryn.events.v1.OneItemTransactionV1`, operation `fee_burn`, oneof tag 6) in the
//! normative ANL-01 `EventEnvelope`.
//!
//! One event per fee transaction: every BURN line (quantity before and after; a whole burn ends
//! `RETIRED` with no location), the closed cause, the fee, the conservation summary, the
//! Character and its committed revision, and the runtime scope. GOLD-FEE-1a burns gold coins
//! only, so the change is always 0 and no change MINT field exists yet (GOLD-FEE-1b adds it).
//! The shape has its own `DUR03-RL-*-FEE-BURN` rows; every bound is checked before encode
//! allocation and oversize input is rejected, never truncated.

use super::item_mint_audit::{
    self as mint, AuditError, EventEnvelopeV1, ITEM_LIFECYCLE_LIVE, OneItemOperationV1,
    OneItemStateV1, OneItemTransactionV1, TransactionEventRefV1, TransactionResourceUsage,
    check_definition, check_uuid_v7, encode_bounded,
};
use super::item_transfer_audit::ITEM_LIFECYCLE_RETIRED;
use crate::domain::charm::CharmKey;
use crate::domain::currency::{COIN_DEFINITION_FAMILY, COIN_STACK_MAXIMUM, Coin, FEE_INPUTS_MAX};
use prost::Message;
use sha2::{Digest, Sha256};

/// Fee-shape rows (decision §4.6).
pub const FEE_RL01_TOUCHED_ITEM_INSTANCES_MAX: u64 = 22;
pub const FEE_RL02_LOCATION_CUSTODY_LINES_MAX: u64 = 22;
pub const FEE_RL06_PARTICIPANTS_MAX: u64 = 22;
pub const FEE_RL06_EFFECT_WORK_UNITS_MAX: u64 = 64;
pub const FEE_RL07_EVENTS_MAX: u64 = 1;
/// Measured worst case of this schema (the `worst_case_*` test), D50 method.
pub const FEE_RL07_PAYLOAD_BYTES_MAX: usize = 24_181;
pub const FEE_RL07_ENVELOPE_BYTES_MAX: usize = 24_495;
/// The largest reachable fee: 20 stacks of 100 crystal coins.
pub const FEE_GOLD_UNITS_MAX: u64 = 20_000_000;

/// `CharmUnassign { charm, occurrence }` (decision §4.4).
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemCharmUnassignV1 {
    #[prost(string, tag = "1")]
    pub charm_key: String,
    #[prost(bytes = "vec", tag = "2")]
    pub occurrence: Vec<u8>,
}

/// The closed `FeeBurnCause`: one variant per admitted fee source.
#[derive(Clone, PartialEq, Eq, prost::Oneof)]
pub enum FeeBurnCauseV1 {
    #[prost(message, tag = "1")]
    CharmUnassign(OneItemCharmUnassignV1),
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemFeeBurnCauseV1 {
    #[prost(oneof = "FeeBurnCauseV1", tags = "1")]
    pub cause: Option<FeeBurnCauseV1>,
}

/// One BURN line: a live coin stack in a direct backpack entry, partly (the last line only) or
/// wholly burned.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemFeeBurnLineV1 {
    #[prost(message, optional, tag = "1")]
    pub before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub after: Option<OneItemStateV1>,
    /// The entry before; for a partial burn also after. A whole burn leaves no location.
    #[prost(uint64, tag = "3")]
    pub placement_ordinal: u64,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemFeeBurnV1 {
    #[prost(message, optional, tag = "1")]
    pub cause: Option<OneItemFeeBurnCauseV1>,
    #[prost(uint64, tag = "2")]
    pub fee_gold_units: u64,
    /// Conservation summary: burned worth; equals the fee while the change is 0.
    #[prost(uint64, tag = "3")]
    pub burned_gold_units: u64,
    #[prost(bytes = "vec", tag = "4")]
    pub character_id: Vec<u8>,
    /// The fee source's committed `CharacterRevision`.
    #[prost(uint64, tag = "5")]
    pub committed_character_revision: u64,
    #[prost(bytes = "vec", tag = "6")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "7")]
    pub channel_id: Vec<u8>,
    #[prost(uint64, tag = "8")]
    pub runtime_scope_ownership_generation: u64,
    /// The equipped main backpack: the parent of every line's entry.
    #[prost(bytes = "vec", tag = "9")]
    pub backpack_item_instance_id: Vec<u8>,
    /// 1..=20 lines in burn order.
    #[prost(message, repeated, tag = "10")]
    pub lines: Vec<OneItemFeeBurnLineV1>,
}

/// Resource usage of one fee transaction: each line is a participant; a whole burn adds its
/// location removal and its retirement, a partial burn its quantity change.
pub fn fee_burn_usage(lines: &[OneItemFeeBurnLineV1]) -> TransactionResourceUsage {
    let whole = lines
        .iter()
        .filter(|line| line.after.as_ref().is_some_and(|after| after.quantity == 0))
        .count() as u64;
    let count = lines.len() as u64;
    TransactionResourceUsage {
        touched_item_instances: count,
        location_custody_lines: whole,
        value_lines: 0,
        transform_lines: 0,
        container_expansion: 0,
        participants: count,
        effect_work_units: 3 * whole + 2 * (count - whole),
        events: 1,
    }
}

fn check_fee_usage(usage: &TransactionResourceUsage) -> Result<(), AuditError> {
    if usage.value_lines != 0 || usage.transform_lines != 0 || usage.container_expansion != 0 {
        return Err(AuditError::InvalidInput);
    }
    if usage.touched_item_instances > FEE_RL01_TOUCHED_ITEM_INSTANCES_MAX
        || usage.location_custody_lines > FEE_RL02_LOCATION_CUSTODY_LINES_MAX
        || usage.participants > FEE_RL06_PARTICIPANTS_MAX
        || usage.effect_work_units > FEE_RL06_EFFECT_WORK_UNITS_MAX
        || usage.events > FEE_RL07_EVENTS_MAX
    {
        return Err(AuditError::CapacityExceeded);
    }
    Ok(())
}

/// The admitted coins of GOLD-FEE-1a: gold only (GOLD-FEE-1b admits platinum and crystal with
/// their change MINT).
fn line_coin(state: &OneItemStateV1) -> Result<Coin, AuditError> {
    let definition = state.definition.as_ref().ok_or(AuditError::InvalidInput)?;
    match Coin::from_production_key(&definition.production_key) {
        Some(Coin::Gold) if definition.family == COIN_DEFINITION_FAMILY => Ok(Coin::Gold),
        _ => Err(AuditError::InvalidInput),
    }
}

/// Complete closed fee shape, conservation and every registered bound.
pub fn check_fee_burn(value: &OneItemFeeBurnV1) -> Result<(), AuditError> {
    let Some(FeeBurnCauseV1::CharmUnassign(cause)) =
        value.cause.as_ref().and_then(|cause| cause.cause.as_ref())
    else {
        return Err(AuditError::InvalidInput);
    };
    CharmKey::new(cause.charm_key.clone()).map_err(|_| AuditError::InvalidInput)?;
    check_uuid_v7(&cause.occurrence)?;
    check_uuid_v7(&value.character_id)?;
    check_uuid_v7(&value.world_id)?;
    check_uuid_v7(&value.channel_id)?;
    check_uuid_v7(&value.backpack_item_instance_id)?;
    if value.lines.len() > FEE_INPUTS_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    if value.lines.is_empty()
        || !(1..=FEE_GOLD_UNITS_MAX).contains(&value.fee_gold_units)
        || value.committed_character_revision < 2
        || value.runtime_scope_ownership_generation == 0
    {
        return Err(AuditError::InvalidInput);
    }
    let mut burned: u64 = 0;
    let mut previous_ordinal = u64::MAX;
    for (index, line) in value.lines.iter().enumerate() {
        let before = line.before.as_ref().ok_or(AuditError::InvalidInput)?;
        let after = line.after.as_ref().ok_or(AuditError::InvalidInput)?;
        check_uuid_v7(&before.item_instance_id)?;
        check_definition(before.definition.as_ref())?;
        let coin = line_coin(before)?;
        let last = index + 1 == value.lines.len();
        let retired = after.lifecycle == ITEM_LIFECYCLE_RETIRED && after.quantity == 0;
        let partial = after.lifecycle == ITEM_LIFECYCLE_LIVE && after.quantity > 0;
        // One coin, worth ascending, then display order (highest ordinal first); only the last
        // line may keep units.
        if before.item_instance_id != after.item_instance_id
            || before.world_id != value.world_id
            || after.world_id != value.world_id
            || before.definition != after.definition
            || before.lifecycle != ITEM_LIFECYCLE_LIVE
            || !(1..=COIN_STACK_MAXIMUM).contains(&before.quantity)
            || after.quantity >= before.quantity
            || !(retired || (partial && last))
            || line.placement_ordinal == 0
            || line.placement_ordinal >= previous_ordinal
            || value.lines[..index].iter().any(|other| {
                other.before.as_ref().map(|b| &b.item_instance_id) == Some(&before.item_instance_id)
            })
        {
            return Err(AuditError::InvalidInput);
        }
        previous_ordinal = line.placement_ordinal;
        burned += u64::from(before.quantity - after.quantity) * coin.worth();
    }
    // Gold only: no change, so the burned worth is exactly the fee and the last line is needed.
    if burned != value.fee_gold_units || value.burned_gold_units != burned {
        return Err(AuditError::InvalidInput);
    }
    check_fee_usage(&fee_burn_usage(&value.lines))
}

/// RL-07 payload gate for the fee shape: size before decode, canonical round trip, closed
/// shape and per-field bounds.
pub fn decode_fee_burn_payload(wire: &[u8]) -> Result<OneItemFeeBurnV1, AuditError> {
    if wire.len() > FEE_RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = OneItemTransactionV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire
        || value.interpretation_revision != mint::INTERPRETATION_REVISION
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::FeeBurn(burn)) = value.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_fee_burn(&burn)?;
    Ok(burn)
}

/// RL-07 envelope gate for the fee shape: the shared envelope gate under the fee envelope row,
/// the payload gate, the envelope scope equal to the payload's and no session, command or
/// causation (the cause carries the source occurrence).
pub fn decode_fee_burn_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, OneItemFeeBurnV1), AuditError> {
    let value = mint::decode_common_envelope_within(wire, FEE_RL07_ENVELOPE_BYTES_MAX)?;
    let burn = decode_fee_burn_payload(&value.payload)?;
    if value.world_id.as_deref() != Some(burn.world_id.as_slice())
        || value.channel_id.as_deref() != Some(burn.channel_id.as_slice())
        || value.game_session_id.is_some()
        || value.command_id.is_some()
        || value.connection_generation.is_some()
        || value.causation.is_some()
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((value, burn))
}

/// Envelope identity fixed before the commit attempt that can become ambiguous.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeBurnEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: &'a str,
}

/// Build the exact immutable event bytes of one fee transaction; re-decoded through both gates
/// before they are returned.
pub fn encode_fee_burn_event(
    identity: FeeBurnEventIdentity<'_>,
    burn: OneItemFeeBurnV1,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    mint::check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    check_fee_burn(&burn)?;
    let (world_id, channel_id) = (burn.world_id.clone(), burn.channel_id.clone());
    let payload = encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::FeeBurn(burn)),
        },
        FEE_RL07_PAYLOAD_BYTES_MAX,
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
    let wire = encode_bounded(&envelope, FEE_RL07_ENVELOPE_BYTES_MAX)?;
    let (decoded, _) = decode_fee_burn_envelope(&wire)?;
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

    fn state(item: u8, revision: &str, quantity: u32) -> OneItemStateV1 {
        OneItemStateV1 {
            item_instance_id: uuid(item),
            world_id: uuid(1),
            definition: Some(OneItemTypedDefinitionRevisionV1 {
                family: COIN_DEFINITION_FAMILY.into(),
                production_key: Coin::Gold.production_key().into(),
                revision_ref: revision.into(),
            }),
            quantity,
            lifecycle: if quantity == 0 {
                ITEM_LIFECYCLE_RETIRED
            } else {
                ITEM_LIFECYCLE_LIVE
            },
        }
    }

    fn line(
        item: u8,
        ordinal: u64,
        before: u32,
        after: u32,
        revision: &str,
    ) -> OneItemFeeBurnLineV1 {
        OneItemFeeBurnLineV1 {
            before: Some(state(item, revision, before)),
            after: Some(state(item, revision, after)),
            placement_ordinal: ordinal,
        }
    }

    fn burn(lines: Vec<OneItemFeeBurnLineV1>, fee: u64) -> OneItemFeeBurnV1 {
        OneItemFeeBurnV1 {
            cause: Some(OneItemFeeBurnCauseV1 {
                cause: Some(FeeBurnCauseV1::CharmUnassign(OneItemCharmUnassignV1 {
                    charm_key: "oteryn:charm.wound".into(),
                    occurrence: uuid(70),
                })),
            }),
            fee_gold_units: fee,
            burned_gold_units: fee,
            character_id: uuid(41),
            committed_character_revision: 2,
            world_id: uuid(1),
            channel_id: uuid(2),
            runtime_scope_ownership_generation: 1,
            backpack_item_instance_id: uuid(50),
            lines,
        }
    }

    fn identity() -> FeeBurnEventIdentity<'static> {
        FeeBurnEventIdentity {
            event_id: fixed(80),
            transaction_id: fixed(81),
            occurred_at_unix_ms: 1_790_000_000_000,
            server_build_id: "oteryn-game-server-test",
        }
    }

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
                .any(|consumer| consumer.as_str().unwrap()
                    == "apps/game-server/src/durability/item_fee_burn_audit.rs"),
            "{id} names the fee runtime consumer"
        );
        rows[0]["hard_maximum"].as_u64().unwrap()
    }

    #[test]
    fn fee_rows_equal_the_registry_and_leave_the_one_item_rows_alone() {
        assert_eq!(
            registered("DUR03-RL-01-FEE-BURN"),
            FEE_RL01_TOUCHED_ITEM_INSTANCES_MAX
        );
        assert_eq!(
            registered("DUR03-RL-02-FEE-BURN"),
            FEE_RL02_LOCATION_CUSTODY_LINES_MAX
        );
        assert_eq!(
            registered("DUR03-RL-06-FEE-BURN-PARTICIPANTS"),
            FEE_RL06_PARTICIPANTS_MAX
        );
        assert_eq!(
            registered("DUR03-RL-06-FEE-BURN-EFFECT-WORK-UNITS"),
            FEE_RL06_EFFECT_WORK_UNITS_MAX
        );
        assert_eq!(
            registered("DUR03-RL-07-FEE-BURN-EVENTS"),
            FEE_RL07_EVENTS_MAX
        );
        assert_eq!(
            registered("DUR03-RL-07-FEE-BURN-PAYLOAD-BYTES"),
            FEE_RL07_PAYLOAD_BYTES_MAX as u64
        );
        assert_eq!(
            registered("DUR03-RL-07-FEE-BURN-ENVELOPE-BYTES"),
            FEE_RL07_ENVELOPE_BYTES_MAX as u64
        );
        // The one-item shapes keep their rows (RL-03 stays 0).
        assert_eq!(mint::RL01_TOUCHED_ITEM_INSTANCES_MAX, 2);
        assert_eq!(mint::RL06_EFFECT_WORK_UNITS_MAX, 6);
        assert_eq!(mint::RL03_VALUE_LINES_MAX, 0);
        assert_eq!(mint::RL07_ENVELOPE_BYTES_MAX, 9_216);
    }

    #[test]
    fn a_whole_and_a_partial_burn_round_trip() {
        let value = burn(
            vec![line(10, 7, 30, 0, "rev-1"), line(11, 3, 50, 45, "rev-1")],
            35,
        );
        let wire = encode_fee_burn_event(identity(), value.clone()).unwrap();
        let (envelope, decoded) = decode_fee_burn_envelope(&wire).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(envelope.game_session_id, None);
        // Other operations' gates refuse the fee payload.
        assert_eq!(
            decode_payload(&envelope.payload),
            Err(AuditError::InvalidInput)
        );
        assert_eq!(
            fee_burn_usage(&value.lines),
            TransactionResourceUsage {
                touched_item_instances: 2,
                location_custody_lines: 1,
                value_lines: 0,
                transform_lines: 0,
                container_expansion: 0,
                participants: 2,
                effect_work_units: 5,
                events: 1,
            }
        );
    }

    #[test]
    fn every_broken_invariant_is_rejected() {
        let good = || burn(vec![line(10, 7, 30, 0, "r"), line(11, 3, 50, 45, "r")], 35);
        let mut cases: Vec<(&str, OneItemFeeBurnV1)> = Vec::new();
        let mut value = good();
        value.fee_gold_units = 36;
        value.burned_gold_units = 36;
        cases.push(("burned worth other than the fee", value));
        let mut value = good();
        value.burned_gold_units = 34;
        cases.push(("summary other than the lines", value));
        let mut value = good();
        value.lines[0].after = Some(state(10, "r", 5));
        value.burned_gold_units = 60;
        value.fee_gold_units = 60;
        cases.push(("a partial line before the last", value));
        let mut value = good();
        value.lines.swap(0, 1);
        cases.push(("display order broken", value));
        let mut value = good();
        value.lines[1].after = Some(state(11, "r", 50));
        cases.push(("a line that burns nothing", value));
        let mut value = good();
        value.lines[1]
            .before
            .as_mut()
            .unwrap()
            .definition
            .as_mut()
            .unwrap()
            .production_key = Coin::Platinum.production_key().into();
        cases.push(("platinum before GOLD-FEE-1b", value));
        let mut value = good();
        value.lines[1]
            .before
            .as_mut()
            .unwrap()
            .definition
            .as_mut()
            .unwrap()
            .production_key = "oteryn:item.tibia.i3032".into();
        cases.push(("not a coin", value));
        let mut value = good();
        value.lines[1]
            .after
            .as_mut()
            .unwrap()
            .definition
            .as_mut()
            .unwrap()
            .revision_ref = "other".into();
        cases.push(("definition changed", value));
        let mut value = good();
        value.lines[0].before = Some(state(10, "r", 101));
        value.lines[0].after = Some(state(10, "r", 0));
        value.fee_gold_units = 106;
        value.burned_gold_units = 106;
        cases.push(("a stack above 100", value));
        let mut value = good();
        value.lines[1].before.as_mut().unwrap().item_instance_id = uuid(10);
        value.lines[1].after.as_mut().unwrap().item_instance_id = uuid(10);
        cases.push(("one item twice", value));
        let mut value = good();
        value.lines[0].after.as_mut().unwrap().world_id = uuid(3);
        cases.push(("World changed", value));
        let mut value = good();
        value.cause = None;
        cases.push(("no cause", value));
        let mut value = good();
        value.cause = Some(OneItemFeeBurnCauseV1 {
            cause: Some(FeeBurnCauseV1::CharmUnassign(OneItemCharmUnassignV1 {
                charm_key: "oteryn:item.wound".into(),
                occurrence: uuid(70),
            })),
        });
        cases.push(("not a charm key", value));
        let mut value = good();
        value.committed_character_revision = 1;
        cases.push(("no Character revision advance", value));
        let mut value = good();
        value.lines.clear();
        value.fee_gold_units = 1;
        value.burned_gold_units = 0;
        cases.push(("no line", value));
        for (case, value) in cases {
            assert_eq!(
                check_fee_burn(&value),
                Err(AuditError::InvalidInput),
                "{case}"
            );
        }
    }

    fn worst_case(lines: usize) -> OneItemFeeBurnV1 {
        let revision = "r".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX);
        let lines = (0..lines)
            .map(|n| {
                let tag = u8::try_from(n).unwrap() + 100;
                line(tag, u64::MAX - u64::from(tag), 100, 0, &revision)
            })
            .collect::<Vec<_>>();
        let mut value = burn(lines, 0);
        let fee = 100 * value.lines.len() as u64;
        value.fee_gold_units = fee;
        value.burned_gold_units = fee;
        value.committed_character_revision = u64::MAX;
        value.runtime_scope_ownership_generation = u64::MAX;
        value.cause = Some(OneItemFeeBurnCauseV1 {
            cause: Some(FeeBurnCauseV1::CharmUnassign(OneItemCharmUnassignV1 {
                charm_key: format!("oteryn:charm.{}", "c".repeat(115)),
                occurrence: uuid(70),
            })),
        });
        value
    }

    // Within the ANL-01 ceilings (payload 196,608 B, envelope 262,144 B).
    const _: () = assert!(FEE_RL07_PAYLOAD_BYTES_MAX <= 196_608);
    const _: () = assert!(FEE_RL07_ENVELOPE_BYTES_MAX <= 262_144);

    #[test]
    fn worst_case_payload_and_envelope_are_the_registered_rows() {
        // 20 whole burns of 100 units, 512-byte revisions, the longest charm key, maximal
        // integers: the exact encoded worst case of this schema.
        let value = worst_case(FEE_INPUTS_MAX);
        let payload = OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::FeeBurn(value.clone())),
        }
        .encoded_len();
        assert_eq!(payload, FEE_RL07_PAYLOAD_BYTES_MAX);
        let identity = FeeBurnEventIdentity {
            occurred_at_unix_ms: i64::MAX,
            server_build_id: &"b".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX),
            ..identity()
        };
        let wire = encode_fee_burn_event(identity, value.clone()).unwrap();
        assert_eq!(wire.len(), FEE_RL07_ENVELOPE_BYTES_MAX);
        // max + 1: a 21st line is rejected before encoding.
        let mut over = worst_case(FEE_INPUTS_MAX + 1);
        over.fee_gold_units = FEE_GOLD_UNITS_MAX;
        assert_eq!(
            encode_fee_burn_event(identity, over),
            Err(AuditError::CapacityExceeded)
        );
        // One byte over either ceiling is refused before decoding.
        let mut long = wire.clone();
        long.push(0);
        assert_eq!(
            decode_fee_burn_envelope(&long),
            Err(AuditError::CapacityExceeded)
        );
        assert_eq!(
            decode_fee_burn_payload(&vec![0; FEE_RL07_PAYLOAD_BYTES_MAX + 1]),
            Err(AuditError::CapacityExceeded)
        );
    }

    #[test]
    fn fee_usage_rows_admit_max_and_reject_max_plus_one() {
        let mut usage = TransactionResourceUsage {
            touched_item_instances: FEE_RL01_TOUCHED_ITEM_INSTANCES_MAX,
            location_custody_lines: FEE_RL02_LOCATION_CUSTODY_LINES_MAX,
            value_lines: 0,
            transform_lines: 0,
            container_expansion: 0,
            participants: FEE_RL06_PARTICIPANTS_MAX,
            effect_work_units: FEE_RL06_EFFECT_WORK_UNITS_MAX,
            events: 1,
        };
        assert_eq!(check_fee_usage(&usage), Ok(()));
        for field in 0..5 {
            let mut over = usage;
            match field {
                0 => over.touched_item_instances += 1,
                1 => over.location_custody_lines += 1,
                2 => over.participants += 1,
                3 => over.effect_work_units += 1,
                _ => over.events += 1,
            }
            assert_eq!(check_fee_usage(&over), Err(AuditError::CapacityExceeded));
        }
        usage.value_lines = 1;
        assert_eq!(check_fee_usage(&usage), Err(AuditError::InvalidInput));
        // The one-item rows still refuse a fee-sized transaction.
        let fee = fee_burn_usage(&worst_case(FEE_INPUTS_MAX).lines);
        assert_eq!(fee.effect_work_units, 60);
        assert_eq!(fee.check(), Err(AuditError::CapacityExceeded));
    }
}
