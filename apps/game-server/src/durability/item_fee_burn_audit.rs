//! Registered DUR-03 fee BURN durable-audit encoding (GOLD-FEE-1a; decision
//! `CHARACTER-GOLD-FEE-BOUNDARY-V1` §4.5, D174-D178; DUR-03 §39.3 gold fee amendment): event
//! type 2 (`oteryn.events.v1.OneItemTransactionV1`, operation `fee_burn`, oneof tag 6) in the
//! normative ANL-01 `EventEnvelope`.
//!
//! One event per fee transaction: every BURN line (quantity before and after; a whole burn ends
//! `RETIRED` with no location), the closed cause, the fee, the conservation summary, the
//! Character and its committed revision, and the runtime scope. GOLD-FEE-1b adds platinum and
//! crystal inputs and the change: its worth and at most two change MINT lines (a fresh platinum
//! stack, then a fresh gold stack, each in a new backpack entry). The shape has its own `DUR03-RL-*-FEE-BURN` rows; every bound is checked before encode
//! allocation and oversize input is rejected, never truncated.
//!
//! GOLD-FEE-2 (BANK-FEE-0 §4.3, ARCH-BATCH-ROOT-PACKETS-V1 §1.7, §2.5) adds the bank part: field
//! 13, the one admitted value line [`OneItemFeeBankDebitV1`] (`FEE_DEBIT`, class BURN), present
//! exactly when a fee burns every eligible stack whole and debits the rest from the payer's
//! (Account, World) balance. The conservation becomes `burned - change + bank_debit = fee` and
//! the 20,000,000 cap applies to the coin part only. A value line is admitted only under the
//! `(2, V2)` tuple ([`Type2EventTuple`]); a `(1, V1)` event carrying one is refused.

use super::bank_audit::{ASSET_GOLD, BANK_BALANCE_MAX};
use super::item_mint_audit::{
    self as mint, AuditError, EventEnvelopeV1, ITEM_LIFECYCLE_LIVE, OneItemOperationV1,
    OneItemStateV1, OneItemTransactionV1, TransactionEventRefV1, TransactionResourceUsage,
    Type2EventTuple, check_definition, check_uuid_v7, encode_bounded,
};
use super::item_transfer_audit::ITEM_LIFECYCLE_RETIRED;
use crate::domain::charm::CharmKey;
use crate::domain::currency::{
    COIN_DEFINITION_FAMILY, COIN_STACK_MAXIMUM, Coin, FEE_CHANGE_OUTPUTS_MAX, FEE_INPUTS_MAX,
};
use prost::Message;
use sha2::{Digest, Sha256};

/// Fee-shape rows (decision §4.6).
pub const FEE_RL01_TOUCHED_ITEM_INSTANCES_MAX: u64 = 22;
pub const FEE_RL02_LOCATION_CUSTODY_LINES_MAX: u64 = 22;
pub const FEE_RL06_PARTICIPANTS_MAX: u64 = 22;
pub const FEE_RL06_EFFECT_WORK_UNITS_MAX: u64 = 64;
/// `DUR03-RL-03-FEE`: the bank part's one value line (BANK-FEE-0 §4.3).
pub const FEE_RL03_VALUE_LINES_MAX: u64 = 1;
pub const FEE_RL07_EVENTS_MAX: u64 = 1;
/// Measured worst case of this schema (the `worst_case_*` test), D50 method.
pub const FEE_RL07_PAYLOAD_BYTES_MAX: usize = 25_398;
pub const FEE_RL07_ENVELOPE_BYTES_MAX: usize = 25_712;
/// The largest coin part of a fee: 20 stacks of 100 crystal coins. A fee paid from coins only
/// is at most this.
pub const FEE_GOLD_UNITS_MAX: u64 = 20_000_000;
/// The largest fee with a bank part: every eligible stack plus `BANK0-RL-01` (BANK-FEE-0 §3).
pub const FEE_WITH_BANK_GOLD_UNITS_MAX: u64 = FEE_GOLD_UNITS_MAX + BANK_BALANCE_MAX;
/// The `FEE_DEBIT` ledger kind, as stored (BANK-FEE-0 §4.2).
pub const LEDGER_FEE_DEBIT: u32 = 5;
/// DUR-03 §17 class of the bank part: value leaves under the fee's `FeeBurnCause`.
pub const LINE_CLASS_BURN: u32 = 3;

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

/// One change MINT (decision §4.2 step 4): a fresh live coin stack that did not exist before,
/// in a new direct backpack entry.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemFeeChangeMintV1 {
    #[prost(message, optional, tag = "1")]
    pub after: Option<OneItemStateV1>,
    #[prost(uint64, tag = "2")]
    pub placement_ordinal: u64,
}

/// The bank part of a fee (BANK-0 §5's closed value line for this one shape): the `FEE_DEBIT`
/// ledger entry, the payer's historical Account resolved under the fee transaction's lock, and
/// the balance before and after.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemFeeBankDebitV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub entry_id: Vec<u8>,
    #[prost(string, tag = "2")]
    pub asset: String,
    #[prost(bytes = "vec", tag = "3")]
    pub account_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub world_id: Vec<u8>,
    #[prost(uint32, tag = "5")]
    pub kind: u32,
    #[prost(uint32, tag = "6")]
    pub line_class: u32,
    #[prost(uint64, tag = "7")]
    pub debit_gold_units: u64,
    #[prost(uint64, tag = "8")]
    pub balance_before_gold_units: u64,
    #[prost(uint64, tag = "9")]
    pub balance_after_gold_units: u64,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemFeeBurnV1 {
    #[prost(message, optional, tag = "1")]
    pub cause: Option<OneItemFeeBurnCauseV1>,
    #[prost(uint64, tag = "2")]
    pub fee_gold_units: u64,
    /// Conservation summary: burned worth, equal to the fee plus the change minus the bank part.
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
    /// The equipped main backpack: the parent of every line's entry. Empty only for a fee paid
    /// wholly from the bank by a payer with no backpack.
    #[prost(bytes = "vec", tag = "9")]
    pub backpack_item_instance_id: Vec<u8>,
    /// 0..=20 lines in burn order; 0 only for a fee paid wholly from the bank.
    #[prost(message, repeated, tag = "10")]
    pub lines: Vec<OneItemFeeBurnLineV1>,
    /// Burned worth minus the fee, below the worth of the last line's coin.
    #[prost(uint64, tag = "11")]
    pub change_gold_units: u64,
    /// 0..=2 change MINT lines: `change / 100` platinum, then `change % 100` gold, each only when
    /// positive.
    #[prost(message, repeated, tag = "12")]
    pub change: Vec<OneItemFeeChangeMintV1>,
    /// The bank part, present exactly when it is positive: every line whole, no change.
    #[prost(message, optional, tag = "13")]
    pub bank_debit: Option<OneItemFeeBankDebitV1>,
}

/// Resource usage of one fee transaction: each line and each change MINT is a participant; a
/// whole burn adds its location removal and its retirement, a partial burn its quantity change,
/// a change MINT its new entry. The bank part is one value line and one participant whose work
/// is the participant and its balance change.
pub fn fee_burn_usage(value: &OneItemFeeBurnV1) -> TransactionResourceUsage {
    let whole = value
        .lines
        .iter()
        .filter(|line| line.after.as_ref().is_some_and(|after| after.quantity == 0))
        .count() as u64;
    let count = value.lines.len() as u64;
    let minted = value.change.len() as u64;
    let bank = u64::from(value.bank_debit.is_some());
    TransactionResourceUsage {
        touched_item_instances: count + minted,
        location_custody_lines: whole + minted,
        value_lines: bank,
        transform_lines: 0,
        container_expansion: 0,
        participants: count + minted + bank,
        effect_work_units: 3 * whole + 2 * (count - whole) + 2 * minted + 2 * bank,
        events: 1,
    }
}

fn check_fee_usage(usage: &TransactionResourceUsage) -> Result<(), AuditError> {
    if usage.value_lines > FEE_RL03_VALUE_LINES_MAX
        || usage.transform_lines != 0
        || usage.container_expansion != 0
    {
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

/// The coin of a line or change state: one of the closed worth table's definitions.
fn line_coin(state: &OneItemStateV1) -> Result<Coin, AuditError> {
    let definition = state.definition.as_ref().ok_or(AuditError::InvalidInput)?;
    match Coin::from_production_key(&definition.production_key) {
        Some(coin) if definition.family == COIN_DEFINITION_FAMILY => Ok(coin),
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
    // Only a fee paid wholly from the bank may name no backpack.
    if !(value.lines.is_empty() && value.backpack_item_instance_id.is_empty()) {
        check_uuid_v7(&value.backpack_item_instance_id)?;
    }
    if value.lines.len() > FEE_INPUTS_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let bank = match &value.bank_debit {
        Some(debit) => {
            check_bank_debit(debit, &value.world_id)?;
            debit.debit_gold_units
        }
        None => 0,
    };
    if (value.lines.is_empty() && bank == 0)
        || value.fee_gold_units == 0
        || value.committed_character_revision < 2
        || value.runtime_scope_ownership_generation == 0
    {
        return Err(AuditError::InvalidInput);
    }
    let mut burned: u64 = 0;
    let mut previous: Option<(u64, u64)> = None;
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
        let ordered = previous.is_none_or(|(worth, ordinal)| {
            coin.worth() > worth || (coin.worth() == worth && line.placement_ordinal < ordinal)
        });
        if !ordered
            || before.item_instance_id != after.item_instance_id
            || before.world_id != value.world_id
            || after.world_id != value.world_id
            || before.definition != after.definition
            || before.lifecycle != ITEM_LIFECYCLE_LIVE
            || !(1..=COIN_STACK_MAXIMUM).contains(&before.quantity)
            || after.quantity >= before.quantity
            || !(retired || (partial && last))
            || line.placement_ordinal == 0
            || value.lines[..index].iter().any(|other| {
                other.before.as_ref().map(|b| &b.item_instance_id) == Some(&before.item_instance_id)
            })
        {
            return Err(AuditError::InvalidInput);
        }
        previous = Some((coin.worth(), line.placement_ordinal));
        burned += u64::from(before.quantity - after.quantity) * coin.worth();
    }
    // Conservation: burned - change + bank_debit = fee; change below the last coin's worth, so
    // every line is needed and the last burns no unit more than the plan. Coins first: a bank
    // part burns every line whole and mints no change.
    let change_bound = previous.map_or(1, |(worth, _)| worth);
    let whole = value
        .lines
        .iter()
        .all(|line| line.after.as_ref().is_some_and(|after| after.quantity == 0));
    if value.burned_gold_units != burned
        || burned
            .checked_sub(value.change_gold_units)
            .and_then(|paid| paid.checked_add(bank))
            != Some(value.fee_gold_units)
        || value.change_gold_units >= change_bound
        || (bank > 0 && (value.change_gold_units != 0 || !whole))
    {
        return Err(AuditError::InvalidInput);
    }
    check_change(value)?;
    check_fee_usage(&fee_burn_usage(value))
}

/// The bank part's closed value line: `FEE_DEBIT`, class BURN, gold, in the fee's World, a
/// positive debit within `BANK0-RL-01` and `after = before - debit`.
fn check_bank_debit(debit: &OneItemFeeBankDebitV1, world_id: &[u8]) -> Result<(), AuditError> {
    check_uuid_v7(&debit.entry_id)?;
    check_uuid_v7(&debit.account_id)?;
    if debit.asset != ASSET_GOLD
        || debit.world_id != world_id
        || debit.kind != LEDGER_FEE_DEBIT
        || debit.line_class != LINE_CLASS_BURN
        || !(1..=BANK_BALANCE_MAX).contains(&debit.debit_gold_units)
        || debit.balance_before_gold_units > BANK_BALANCE_MAX
        || debit
            .balance_before_gold_units
            .checked_sub(debit.debit_gold_units)
            != Some(debit.balance_after_gold_units)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

/// The stored `FEE_DEBIT` ledger entry a fee event's bank part must repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeDebitEntryFacts {
    pub entry_id: [u8; 16],
    pub account_id: [u8; 16],
    pub world_id: [u8; 16],
    pub amount: u64,
    pub balance_before: u64,
    pub balance_after: u64,
}

/// A fee event's bank part names exactly its `FEE_DEBIT` entry: the same entry, Account, World,
/// amount and balances. An event without a bank part never matches an entry.
pub fn check_bank_debit_matches_entry(
    burn: &OneItemFeeBurnV1,
    entry: &FeeDebitEntryFacts,
) -> Result<(), AuditError> {
    let debit = burn.bank_debit.as_ref().ok_or(AuditError::InvalidInput)?;
    if debit.entry_id != entry.entry_id
        || debit.account_id != entry.account_id
        || debit.world_id != entry.world_id
        || burn.world_id != entry.world_id
        || debit.debit_gold_units != entry.amount
        || debit.balance_before_gold_units != entry.balance_before
        || debit.balance_after_gold_units != entry.balance_after
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

/// Change MINT lines: `change / 100` platinum then `change % 100` gold, each only when
/// positive, fresh live items of this World in consecutive new entries after the burn lines.
fn check_change(value: &OneItemFeeBurnV1) -> Result<(), AuditError> {
    let expected: Vec<(Coin, u64)> = [
        (Coin::Platinum, value.change_gold_units / 100),
        (Coin::Gold, value.change_gold_units % 100),
    ]
    .into_iter()
    .filter(|(_, quantity)| *quantity > 0)
    .collect();
    if value.change.len() > FEE_CHANGE_OUTPUTS_MAX || value.change.len() != expected.len() {
        return Err(AuditError::InvalidInput);
    }
    // The first output takes an ordinal after every burn line's entry, the next one after it.
    let last_line_ordinal = value.lines.iter().map(|line| line.placement_ordinal).max();
    let mut previous_ordinal: Option<u64> = None;
    for (output, (coin, quantity)) in value.change.iter().zip(expected) {
        let after = output.after.as_ref().ok_or(AuditError::InvalidInput)?;
        check_uuid_v7(&after.item_instance_id)?;
        check_definition(after.definition.as_ref())?;
        let consecutive = match previous_ordinal {
            Some(ordinal) => ordinal.checked_add(1) == Some(output.placement_ordinal),
            None => last_line_ordinal.is_none_or(|ordinal| output.placement_ordinal > ordinal),
        };
        if line_coin(after)? != coin
            || u64::from(after.quantity) != quantity
            || after.lifecycle != ITEM_LIFECYCLE_LIVE
            || after.world_id != value.world_id
            || output.placement_ordinal == 0
            || !consecutive
            || value.lines.iter().any(|line| {
                line.before.as_ref().map(|b| &b.item_instance_id) == Some(&after.item_instance_id)
            })
            || value
                .change
                .iter()
                .filter(|other| {
                    other.after.as_ref().map(|a| &a.item_instance_id)
                        == Some(&after.item_instance_id)
                })
                .count()
                != 1
        {
            return Err(AuditError::InvalidInput);
        }
        previous_ordinal = Some(output.placement_ordinal);
    }
    Ok(())
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

/// RL-07 envelope gate for the fee shape: the shared envelope gate under the fee envelope row
/// (either admitted tuple), the payload gate, a bank part only under `(2, V2)`, the envelope
/// scope equal to the payload's and no session, command or causation (the cause carries the
/// source occurrence).
pub fn decode_fee_burn_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, OneItemFeeBurnV1), AuditError> {
    let value = mint::decode_common_envelope_within(wire, FEE_RL07_ENVELOPE_BYTES_MAX)?;
    let burn = decode_fee_burn_payload(&value.payload)?;
    let tuple = Type2EventTuple::of(value.event_schema_revision, &value.retention_profile_id)
        .ok_or(AuditError::InvalidInput)?;
    if (tuple == Type2EventTuple::V1 && burn.bank_debit.is_some())
        || value.world_id.as_deref() != Some(burn.world_id.as_slice())
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

/// Build the exact immutable event bytes of one fee transaction under the `(1, V1)` tuple every
/// type-2 producer emits in phase 1; re-decoded through both gates before they are returned.
pub fn encode_fee_burn_event(
    identity: FeeBurnEventIdentity<'_>,
    burn: OneItemFeeBurnV1,
) -> Result<Vec<u8>, AuditError> {
    encode_fee_burn_event_with(identity, burn, Type2EventTuple::V1)
}

/// [`encode_fee_burn_event`] under an explicit tuple. A bank part needs `(2, V2)`.
pub fn encode_fee_burn_event_with(
    identity: FeeBurnEventIdentity<'_>,
    burn: OneItemFeeBurnV1,
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    mint::check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    check_fee_burn(&burn)?;
    if tuple == Type2EventTuple::V1 && burn.bank_debit.is_some() {
        return Err(AuditError::InvalidInput);
    }
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
        event_schema_revision: tuple.schema_revision(),
        durability_class: mint::DURABLE_AUDIT,
        privacy_class: mint::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
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
        coin_state(Coin::Gold, item, revision, quantity)
    }

    fn coin_state(coin: Coin, item: u8, revision: &str, quantity: u32) -> OneItemStateV1 {
        OneItemStateV1 {
            item_instance_id: uuid(item),
            world_id: uuid(1),
            definition: Some(OneItemTypedDefinitionRevisionV1 {
                family: COIN_DEFINITION_FAMILY.into(),
                production_key: coin.production_key().into(),
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
            change_gold_units: 0,
            change: Vec::new(),
            bank_debit: None,
        }
    }

    fn coin_line(
        coin: Coin,
        item: u8,
        ordinal: u64,
        before: u32,
        after: u32,
    ) -> OneItemFeeBurnLineV1 {
        OneItemFeeBurnLineV1 {
            before: Some(coin_state(coin, item, "r", before)),
            after: Some(coin_state(coin, item, "r", after)),
            placement_ordinal: ordinal,
        }
    }

    fn minted(coin: Coin, item: u8, quantity: u32, ordinal: u64) -> OneItemFeeChangeMintV1 {
        OneItemFeeChangeMintV1 {
            after: Some(coin_state(coin, item, "r", quantity)),
            placement_ordinal: ordinal,
        }
    }

    /// The decision §8 example: 2,350 from 10 gold and one crystal mints 76 platinum and 60
    /// gold into two new entries.
    fn with_change() -> OneItemFeeBurnV1 {
        let mut value = burn(
            vec![
                coin_line(Coin::Gold, 10, 1, 10, 0),
                coin_line(Coin::Crystal, 11, 2, 1, 0),
            ],
            2_350,
        );
        value.burned_gold_units = 10_010;
        value.change_gold_units = 7_660;
        value.change = vec![
            minted(Coin::Platinum, 20, 76, 3),
            minted(Coin::Gold, 21, 60, 4),
        ];
        value
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
        assert_eq!(registered("DUR03-RL-03-FEE"), FEE_RL03_VALUE_LINES_MAX);
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
            fee_burn_usage(&value),
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
        value.lines[0] = coin_line(Coin::Platinum, 10, 7, 1, 0);
        value.burned_gold_units = 105;
        value.fee_gold_units = 105;
        cases.push(("platinum burned before gold", value));
        let mut value = good();
        value.change_gold_units = 1;
        value.burned_gold_units = 36;
        cases.push(("change without its MINT", value));
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

    #[test]
    fn a_change_mint_round_trips_and_every_broken_change_is_rejected() {
        let value = with_change();
        let wire = encode_fee_burn_event(identity(), value.clone()).unwrap();
        assert_eq!(decode_fee_burn_envelope(&wire).unwrap().1, value);
        // An untouched entry may hold a higher ordinal than every burn line.
        let mut later = with_change();
        later.change[0].placement_ordinal = 7;
        later.change[1].placement_ordinal = 8;
        assert!(encode_fee_burn_event(identity(), later).is_ok());
        let usage = fee_burn_usage(&value);
        assert_eq!(
            (usage.touched_item_instances, usage.location_custody_lines),
            (4, 4)
        );
        assert_eq!(usage.effect_work_units, 10);

        let mut cases: Vec<(&str, OneItemFeeBurnV1)> = Vec::new();
        let mut broken = with_change();
        broken.change.swap(0, 1);
        cases.push(("gold before platinum", broken));
        let mut broken = with_change();
        broken.change[1] = minted(Coin::Gold, 21, 59, 4);
        cases.push(("change quantity other than change mod 100", broken));
        let mut broken = with_change();
        broken.change.pop();
        cases.push(("a change output missing", broken));
        let mut broken = with_change();
        broken.change[1].placement_ordinal = 5;
        cases.push(("change entries not consecutive", broken));
        let mut broken = with_change();
        broken.change[0].placement_ordinal = 2;
        broken.change[1].placement_ordinal = 3;
        cases.push(("change entry at a burn line's ordinal", broken));
        let mut broken = with_change();
        broken.change[0].placement_ordinal = 1;
        broken.change[1].placement_ordinal = 2;
        cases.push(("change entries before the burn lines", broken));
        let mut broken = with_change();
        broken.change[1] = minted(Coin::Gold, 11, 60, 4);
        cases.push(("change reuses a burned item", broken));
        let mut broken = with_change();
        broken.change[1] = minted(Coin::Gold, 20, 60, 4);
        cases.push(("one item minted twice", broken));
        let mut broken = with_change();
        broken.change[0] = minted(Coin::Crystal, 20, 76, 3);
        cases.push(("crystal change", broken));
        let mut broken = with_change();
        broken.change[0].after.as_mut().unwrap().world_id = uuid(3);
        cases.push(("change in another World", broken));
        // Burning a second crystal would overpay: change of 10,000 or more is never planned.
        let mut broken = with_change();
        broken.lines[1] = coin_line(Coin::Crystal, 11, 2, 2, 0);
        broken.burned_gold_units = 20_010;
        broken.change_gold_units = 17_660;
        cases.push(("change at or above the last coin's worth", broken));
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
                line(tag, u64::MAX - 2 - u64::from(tag), 100, 0, &revision)
            })
            .collect::<Vec<_>>();
        let mut value = burn(lines, 0);
        // Every line whole and the last one a crystal stack paying 9,999 more than the fee: the
        // largest change (99 platinum, 99 gold) in two maximal outputs.
        let last = value.lines.len() - 1;
        let last_line = &mut value.lines[last];
        for state in [&mut last_line.before, &mut last_line.after] {
            state
                .as_mut()
                .unwrap()
                .definition
                .as_mut()
                .unwrap()
                .production_key = Coin::Crystal.production_key().into();
        }
        let burned = 100 * last as u64 + 100 * Coin::Crystal.worth();
        value.burned_gold_units = burned;
        value.change_gold_units = 9_999;
        value.fee_gold_units = burned - 9_999;
        value.change = [(Coin::Platinum, 99), (Coin::Gold, 99)]
            .into_iter()
            .enumerate()
            .map(|(n, (coin, quantity))| OneItemFeeChangeMintV1 {
                after: Some(coin_state(
                    coin,
                    200 + u8::try_from(n).unwrap(),
                    &revision,
                    quantity,
                )),
                placement_ordinal: u64::MAX - 1 + n as u64,
            })
            .collect();
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
        // 20 whole burns of 100 units, two change MINTs of 99, 512-byte revisions, the longest
        // charm key, maximal integers: the exact encoded worst case of this schema.
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
        // DUR03-RL-03-FEE: the one bank value line is admitted, a second is not.
        usage.value_lines = FEE_RL03_VALUE_LINES_MAX;
        assert_eq!(check_fee_usage(&usage), Ok(()));
        usage.value_lines = FEE_RL03_VALUE_LINES_MAX + 1;
        assert_eq!(check_fee_usage(&usage), Err(AuditError::InvalidInput));
        // The one-item rows still refuse a fee-sized transaction.
        let fee = fee_burn_usage(&worst_case(FEE_INPUTS_MAX));
        assert_eq!(fee.effect_work_units, FEE_RL06_EFFECT_WORK_UNITS_MAX);
        assert_eq!(
            fee.touched_item_instances,
            FEE_RL01_TOUCHED_ITEM_INSTANCES_MAX
        );
        assert_eq!(fee.check(), Err(AuditError::CapacityExceeded));
    }

    fn debit(debit: u64, before: u64) -> OneItemFeeBankDebitV1 {
        OneItemFeeBankDebitV1 {
            entry_id: uuid(90),
            asset: ASSET_GOLD.into(),
            account_id: uuid(91),
            world_id: uuid(1),
            kind: LEDGER_FEE_DEBIT,
            line_class: LINE_CLASS_BURN,
            debit_gold_units: debit,
            balance_before_gold_units: before,
            balance_after_gold_units: before - debit,
        }
    }

    /// 30 + 50 gold burned whole and 920 from the bank pay a fee of 1,000.
    fn coins_then_bank() -> OneItemFeeBurnV1 {
        let mut value = burn(
            vec![line(10, 7, 30, 0, "r"), line(11, 3, 50, 0, "r")],
            1_000,
        );
        value.burned_gold_units = 80;
        value.bank_debit = Some(debit(920, 5_000));
        value
    }

    /// No backpack: the whole fee from the bank.
    fn bank_only() -> OneItemFeeBurnV1 {
        let mut value = burn(Vec::new(), 1_000);
        value.burned_gold_units = 0;
        value.backpack_item_instance_id = Vec::new();
        value.bank_debit = Some(debit(1_000, 1_000));
        value
    }

    fn v2_envelope_tuple(wire: &[u8]) -> (u32, String) {
        let envelope = EventEnvelopeV1::decode(wire).unwrap();
        (
            envelope.event_schema_revision,
            envelope.retention_profile_id,
        )
    }

    #[test]
    fn a_bank_part_round_trips_only_under_the_v2_tuple() {
        for value in [coins_then_bank(), bank_only()] {
            assert_eq!(check_fee_burn(&value), Ok(()));
            let wire =
                encode_fee_burn_event_with(identity(), value.clone(), Type2EventTuple::V2).unwrap();
            let (envelope, decoded) = decode_fee_burn_envelope(&wire).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(
                v2_envelope_tuple(&wire),
                (2, "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2".into())
            );
            // Canonical: the decoded envelope re-encodes to the same bytes.
            assert_eq!(envelope.encode_to_vec(), wire);
            // (1, V1) cannot carry a value line, neither when encoding nor when verifying.
            assert_eq!(
                encode_fee_burn_event(identity(), value.clone()),
                Err(AuditError::InvalidInput)
            );
            let mut as_v1 = envelope.clone();
            as_v1.event_schema_revision = mint::EVENT_SCHEMA_REVISION;
            as_v1.retention_profile_id = mint::RETENTION_PROFILE_ID.into();
            assert_eq!(
                decode_fee_burn_envelope(&as_v1.encode_to_vec()),
                Err(AuditError::InvalidInput)
            );
        }
        // A coin-only fee is the same under either tuple.
        let coins = with_change();
        let v1 = encode_fee_burn_event(identity(), coins.clone()).unwrap();
        let v2 =
            encode_fee_burn_event_with(identity(), coins.clone(), Type2EventTuple::V2).unwrap();
        assert_eq!(decode_fee_burn_envelope(&v1).unwrap().1, coins);
        assert_eq!(decode_fee_burn_envelope(&v2).unwrap().1, coins);
        assert_eq!(
            fee_burn_usage(&coins_then_bank()),
            TransactionResourceUsage {
                touched_item_instances: 2,
                location_custody_lines: 2,
                value_lines: 1,
                transform_lines: 0,
                container_expansion: 0,
                participants: 3,
                effect_work_units: 8,
                events: 1,
            }
        );
        assert_eq!(fee_burn_usage(&bank_only()).value_lines, 1);
        assert_eq!(fee_burn_usage(&with_change()).value_lines, 0);
    }

    #[test]
    fn every_broken_bank_part_is_rejected() {
        let mut cases: Vec<(&str, OneItemFeeBurnV1)> = Vec::new();
        let mut value = coins_then_bank();
        value.bank_debit.as_mut().unwrap().debit_gold_units = 0;
        value.bank_debit.as_mut().unwrap().balance_after_gold_units = 5_000;
        value.fee_gold_units = 80;
        cases.push(("a bank debit of zero", value));
        let mut value = coins_then_bank();
        value.bank_debit.as_mut().unwrap().balance_after_gold_units = 4_081;
        cases.push(("after other than before minus debit", value));
        let mut value = coins_then_bank();
        let short = value.bank_debit.as_mut().unwrap();
        short.balance_before_gold_units = 900;
        short.balance_after_gold_units = 0;
        cases.push(("a debit above the balance", value));
        let mut value = coins_then_bank();
        value.bank_debit.as_mut().unwrap().balance_before_gold_units = BANK_BALANCE_MAX + 920;
        value.bank_debit.as_mut().unwrap().balance_after_gold_units = BANK_BALANCE_MAX;
        cases.push(("a balance above BANK0-RL-01", value));
        for (case, account) in [
            ("a missing AccountId", Vec::new()),
            ("a zero AccountId", vec![0; 16]),
            ("a 15-byte AccountId", uuid(91)[..15].to_vec()),
            ("a 17-byte AccountId", [uuid(91), vec![0]].concat()),
        ] {
            let mut value = coins_then_bank();
            value.bank_debit.as_mut().unwrap().account_id = account;
            cases.push((case, value));
        }
        let mut value = coins_then_bank();
        value.bank_debit.as_mut().unwrap().entry_id = Vec::new();
        cases.push(("no ledger entry", value));
        let mut value = coins_then_bank();
        value.bank_debit.as_mut().unwrap().world_id = uuid(3);
        cases.push(("another World", value));
        let mut value = coins_then_bank();
        value.bank_debit.as_mut().unwrap().asset = "platinum".into();
        cases.push(("not gold", value));
        let mut value = coins_then_bank();
        value.bank_debit.as_mut().unwrap().kind = 2;
        cases.push(("not FEE_DEBIT", value));
        let mut value = coins_then_bank();
        value.bank_debit.as_mut().unwrap().line_class = 1;
        cases.push(("not class BURN", value));
        let mut value = coins_then_bank();
        value.fee_gold_units = 1_001;
        cases.push(("burned plus debit other than the fee", value));
        let mut value = coins_then_bank();
        value.lines[1] = line(11, 3, 50, 10, "r");
        value.burned_gold_units = 70;
        value.bank_debit = Some(debit(930, 5_000));
        cases.push(("a partial line with a bank part", value));
        let mut value = with_change();
        value.bank_debit = Some(debit(1, 5_000));
        value.fee_gold_units += 1;
        cases.push(("change with a bank part", value));
        let mut value = bank_only();
        value.bank_debit = None;
        cases.push(("no line and no bank part", value));
        let mut value = coins_then_bank();
        value.backpack_item_instance_id = Vec::new();
        cases.push(("lines without a backpack", value));
        for (case, value) in cases {
            assert_eq!(
                check_fee_burn(&value),
                Err(AuditError::InvalidInput),
                "{case}"
            );
            assert!(
                encode_fee_burn_event_with(identity(), value, Type2EventTuple::V2).is_err(),
                "{case}"
            );
        }
    }

    #[test]
    fn the_bank_part_names_exactly_its_fee_debit_entry() {
        let value = coins_then_bank();
        let entry = FeeDebitEntryFacts {
            entry_id: fixed(90),
            account_id: fixed(91),
            world_id: fixed(1),
            amount: 920,
            balance_before: 5_000,
            balance_after: 4_080,
        };
        assert_eq!(check_bank_debit_matches_entry(&value, &entry), Ok(()));
        // The AccountId is carried byte for byte.
        let wire =
            encode_fee_burn_event_with(identity(), value.clone(), Type2EventTuple::V2).unwrap();
        let decoded = decode_fee_burn_envelope(&wire).unwrap().1;
        assert_eq!(decoded.bank_debit.unwrap().account_id, uuid(91));
        for (case, broken) in [
            (
                "another Account",
                FeeDebitEntryFacts {
                    account_id: fixed(92),
                    ..entry
                },
            ),
            (
                "another World",
                FeeDebitEntryFacts {
                    world_id: fixed(3),
                    ..entry
                },
            ),
            (
                "another entry",
                FeeDebitEntryFacts {
                    entry_id: fixed(93),
                    ..entry
                },
            ),
            (
                "another amount",
                FeeDebitEntryFacts {
                    amount: 921,
                    balance_after: 4_079,
                    ..entry
                },
            ),
        ] {
            assert_eq!(
                check_bank_debit_matches_entry(&value, &broken),
                Err(AuditError::InvalidInput),
                "{case}"
            );
        }
        assert_eq!(
            check_bank_debit_matches_entry(&with_change(), &entry),
            Err(AuditError::InvalidInput)
        );
    }

    /// The largest bank-part event: 20 whole crystal burns at the longest revision and the largest
    /// debit, fee and balances.
    fn worst_case_with_bank() -> OneItemFeeBurnV1 {
        let revision = "r".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX);
        let lines: Vec<OneItemFeeBurnLineV1> = (0..FEE_INPUTS_MAX)
            .map(|n| {
                let tag = u8::try_from(n).unwrap() + 100;
                let state = |quantity| coin_state(Coin::Crystal, tag, &revision, quantity);
                OneItemFeeBurnLineV1 {
                    before: Some(state(100)),
                    after: Some(state(0)),
                    placement_ordinal: u64::MAX - u64::from(tag),
                }
            })
            .collect();
        let mut value = worst_case(FEE_INPUTS_MAX);
        value.lines = lines;
        value.change_gold_units = 0;
        value.change = Vec::new();
        value.burned_gold_units = FEE_GOLD_UNITS_MAX;
        value.fee_gold_units = FEE_WITH_BANK_GOLD_UNITS_MAX;
        value.bank_debit = Some(debit(BANK_BALANCE_MAX, BANK_BALANCE_MAX));
        value
    }

    #[test]
    fn the_largest_bank_part_fits_the_re_measured_rows() {
        // Re-measured with the value line: a bank part mints no change, so its worst case stays
        // below the change-MINT worst case, which remains the registered maximum.
        let value = worst_case_with_bank();
        assert_eq!(check_fee_burn(&value), Ok(()));
        let payload = OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::FeeBurn(value.clone())),
        }
        .encoded_len();
        let identity = FeeBurnEventIdentity {
            occurred_at_unix_ms: i64::MAX,
            server_build_id: &"b".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX),
            ..identity()
        };
        let wire =
            encode_fee_burn_event_with(identity, value.clone(), Type2EventTuple::V2).unwrap();
        assert_eq!((payload, wire.len()), (24_267, 24_581));
        assert!(payload < FEE_RL07_PAYLOAD_BYTES_MAX);
        assert!(wire.len() < FEE_RL07_ENVELOPE_BYTES_MAX);
        // The largest fee is accepted; one more gold unit cannot be paid by any balance.
        let mut over = value;
        over.fee_gold_units += 1;
        assert_eq!(check_fee_burn(&over), Err(AuditError::InvalidInput));
        let usage = fee_burn_usage(&worst_case_with_bank());
        assert_eq!(usage.value_lines, FEE_RL03_VALUE_LINES_MAX);
        assert_eq!(check_fee_usage(&usage), Ok(()));
    }

    #[test]
    fn a_v1_fee_event_is_byte_identical_to_the_main_codec() {
        // A `main` node verifies a GOLD-FEE-2 node's (1, V1) fee events: the new codec emits the
        // exact bytes `main` emitted for the same fees.
        use super::super::item_mint_audit::golden::{TYPE2_GOLDEN_V1, unhex};
        let golden = |shape: &str| {
            unhex(
                TYPE2_GOLDEN_V1
                    .iter()
                    .find(|(name, _)| *name == shape)
                    .unwrap()
                    .1,
            )
        };
        let partial = burn(
            vec![line(10, 7, 30, 0, "rev-1"), line(11, 3, 50, 45, "rev-1")],
            35,
        );
        assert_eq!(
            encode_fee_burn_event(identity(), partial).unwrap(),
            golden("fee_partial")
        );
        assert_eq!(
            encode_fee_burn_event(identity(), with_change()).unwrap(),
            golden("fee_change")
        );
    }
}
