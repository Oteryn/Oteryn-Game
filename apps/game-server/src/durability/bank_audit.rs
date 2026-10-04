//! Registered bank durable-audit encoding (BANK-1; decision
//! `BANK0-ACCOUNT-WORLD-BANK-BALANCE-V1` §5): event type 3 `BANK_OPERATION`
//! (`oteryn.events.v2.BankOperationV1`, `docs/contracts/game-events/v2/bank_operation.proto`) in
//! the normative ANL-01 `EventEnvelope`, retention `ECONOMY_LEDGER_RETENTION_V1`.
//!
//! One event per committed bank operation: the closed cause (`BankConversionCause` for a deposit
//! or withdrawal, `BankTransferCause` for a transfer), the acting character and runtime scope,
//! the amount, the coin lines of a deposit or withdrawal (every one a `CONVERSION` line) and the
//! value lines (one `CONVERSION` line, or the two `TRANSFER` lines of a transfer). No bank shape
//! has a BURN or a MINT. Every bound is a registered row (`BANK0-RL-*`, `DUR03-RL-*-BANK`),
//! checked before encode allocation; oversize input is rejected, never truncated.

use super::item_mint_audit::{
    self as mint, AuditError, DURABLE_AUDIT, ENVELOPE_REVISION, EventEnvelopeV1,
    ITEM_LIFECYCLE_LIVE, RESTRICTED_PLAYER_LINKED, TransactionEventRefV1, check_content_key,
    check_technical_text, check_uuid_v7, encode_bounded,
};
use super::item_transfer_audit::ITEM_LIFECYCLE_RETIRED;
use crate::domain::currency::{COIN_DEFINITION_FAMILY, COIN_STACK_MAXIMUM, Coin, FEE_INPUTS_MAX};
use prost::Message;
use sha2::{Digest, Sha256};

pub const EVENT_TYPE_ID: u32 = 3;
pub const EVENT_SCHEMA_REVISION: u32 = 1;
pub const INTERPRETATION_REVISION: u32 = 1;
pub const RETENTION_PROFILE_ID: &str = "ECONOMY_LEDGER_RETENTION_V1";
/// `ECONOMY_LEDGER_RETENTION_V1` (P30D) in milliseconds; expiry = occurred_at + this.
pub const RETENTION_P30D_MS: i64 = 2_592_000_000;
/// The only bank asset: gold pieces, converted by the gold fee worth table (BANK-0 §3).
pub const ASSET_GOLD: &str = "gold";

/// `BANK0-RL-01`: the balance maximum, in gold.
pub const BANK_BALANCE_MAX: u64 = 999_999_999_999;
/// `BANK0-RL-02-DEPOSIT`: 20 input stacks of 100 crystal coins.
pub const BANK_DEPOSIT_MAX: u64 = 20_000_000;
/// `BANK0-RL-02-WITHDRAW`: 100 crystal, 99 platinum and 99 gold coins.
pub const BANK_WITHDRAW_MAX: u64 = 1_009_999;
/// `BANK0-RL-02-TRANSFER`.
pub const BANK_TRANSFER_MAX: u64 = BANK_BALANCE_MAX;
/// `BANK0-RL-03`: ledger entries of one operation.
pub const BANK_ENTRIES_MAX: u64 = 2;
/// A deposit's inputs (the gold fee plan's `FEE_INPUTS_MAX`) and change outputs.
pub const BANK_DEPOSIT_INPUTS_MAX: usize = FEE_INPUTS_MAX;
pub const BANK_DEPOSIT_CHANGE_MAX: usize = 2;
/// A withdrawal's outputs: crystal, platinum, gold.
pub const BANK_WITHDRAW_OUTPUTS_MAX: usize = 3;
/// `DUR03-RL-01-BANK`: 22 for a deposit, 3 for a withdrawal, 0 for a transfer.
pub const BANK_RL01_TOUCHED_ITEM_INSTANCES_MAX: u64 = 22;
/// `DUR03-RL-03-BANK`: 1, or 2 for a transfer.
pub const BANK_RL03_VALUE_LINES_MAX: u64 = 2;
/// Measured worst case of this schema (the `worst_case_*` test), D50 method.
pub const BANK_RL07_PAYLOAD_BYTES_MAX: usize = 25_419;
pub const BANK_RL07_ENVELOPE_BYTES_MAX: usize = 25_719;
// Inherited ANL-01 envelope string ceiling.
const ANL_ENVELOPE_STRING_MAX: usize = 128;

pub const COIN_LINE_INPUT: u32 = 1;
pub const COIN_LINE_OUTPUT: u32 = 2;
/// DUR-03 §17 line classes of the bank shapes.
pub const LINE_CLASS_CONVERSION: u32 = 1;
pub const LINE_CLASS_TRANSFER: u32 = 2;
/// Ledger kinds, as stored.
pub const LEDGER_DEPOSIT: u32 = 1;
pub const LEDGER_WITHDRAW: u32 = 2;
pub const LEDGER_TRANSFER_OUT: u32 = 3;
pub const LEDGER_TRANSFER_IN: u32 = 4;
/// `BankConversionCause` kinds.
pub const CONVERSION_DEPOSIT: u32 = 1;
pub const CONVERSION_WITHDRAW: u32 = 2;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct BankItemDefinitionV1 {
    #[prost(string, tag = "1")]
    pub family: String,
    #[prost(string, tag = "2")]
    pub production_key: String,
    #[prost(string, tag = "3")]
    pub revision_ref: String,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct BankItemStateV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub item_instance_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub world_id: Vec<u8>,
    #[prost(message, optional, tag = "3")]
    pub definition: Option<BankItemDefinitionV1>,
    #[prost(uint32, tag = "4")]
    pub quantity: u32,
    #[prost(uint32, tag = "5")]
    pub lifecycle: u32,
}

/// One coin line: an input consumed by a deposit (before and after) or an output a deposit's
/// change or a withdrawal creates (after only), in a direct entry of the main backpack.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct BankCoinLineV1 {
    #[prost(uint32, tag = "1")]
    pub direction: u32,
    #[prost(message, optional, tag = "2")]
    pub before: Option<BankItemStateV1>,
    #[prost(message, optional, tag = "3")]
    pub after: Option<BankItemStateV1>,
    #[prost(uint64, tag = "4")]
    pub placement_ordinal: u64,
    #[prost(uint32, tag = "5")]
    pub line_class: u32,
}

/// The closed value line (BANK-0 §5): one balance change of one ledger entry.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct BankValueLineV1 {
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
    pub amount: u64,
    #[prost(uint64, tag = "8")]
    pub balance_before: u64,
    #[prost(uint64, tag = "9")]
    pub balance_after: u64,
}

/// `BankConversionCause { Deposit | Withdraw, occurrence }`.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct BankConversionCauseV1 {
    #[prost(uint32, tag = "1")]
    pub kind: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub occurrence: Vec<u8>,
}

/// `BankTransferCause { occurrence }`.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct BankTransferCauseV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub occurrence: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, prost::Oneof)]
pub enum BankCauseV1 {
    #[prost(message, tag = "1")]
    Conversion(BankConversionCauseV1),
    #[prost(message, tag = "2")]
    Transfer(BankTransferCauseV1),
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct BankOperationCauseV1 {
    #[prost(oneof = "BankCauseV1", tags = "1, 2")]
    pub cause: Option<BankCauseV1>,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct BankOperationV1 {
    #[prost(uint32, tag = "1")]
    pub interpretation_revision: u32,
    #[prost(message, optional, tag = "2")]
    pub cause: Option<BankOperationCauseV1>,
    #[prost(bytes = "vec", tag = "3")]
    pub acting_character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub channel_id: Vec<u8>,
    #[prost(uint64, tag = "6")]
    pub runtime_scope_ownership_generation: u64,
    #[prost(uint64, tag = "7")]
    pub amount: u64,
    /// The main backpack of a deposit or withdrawal; empty for a transfer.
    #[prost(bytes = "vec", tag = "8")]
    pub backpack_item_instance_id: Vec<u8>,
    /// Inputs first (plan order), then outputs (worth descending, consecutive new entries).
    #[prost(message, repeated, tag = "9")]
    pub coin_lines: Vec<BankCoinLineV1>,
    #[prost(message, repeated, tag = "10")]
    pub value_lines: Vec<BankValueLineV1>,
    /// The recipient of a transfer; empty otherwise.
    #[prost(bytes = "vec", tag = "11")]
    pub recipient_character_id: Vec<u8>,
}

/// The coin of a line state: one of the closed worth table's definitions.
fn state_coin(state: &BankItemStateV1) -> Result<Coin, AuditError> {
    let definition = state.definition.as_ref().ok_or(AuditError::InvalidInput)?;
    check_technical_text(&definition.family)?;
    check_content_key(&definition.production_key)?;
    check_content_key(&definition.revision_ref)?;
    match Coin::from_production_key(&definition.production_key) {
        Some(coin) if definition.family == COIN_DEFINITION_FAMILY => Ok(coin),
        _ => Err(AuditError::InvalidInput),
    }
}

/// The value line of one balance change.
fn check_value_line(
    line: &BankValueLineV1,
    world_id: &[u8],
    kind: u32,
    class: u32,
    amount: u64,
) -> Result<(), AuditError> {
    check_uuid_v7(&line.entry_id)?;
    check_uuid_v7(&line.account_id)?;
    let after = match kind {
        LEDGER_DEPOSIT | LEDGER_TRANSFER_IN => line.balance_before.checked_add(amount),
        _ => line.balance_before.checked_sub(amount),
    };
    if line.asset != ASSET_GOLD
        || line.world_id != world_id
        || line.kind != kind
        || line.line_class != class
        || line.amount != amount
        || after != Some(line.balance_after)
        || line.balance_before > BANK_BALANCE_MAX
        || line.balance_after > BANK_BALANCE_MAX
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

/// The coin each output line must be and its quantity: a deposit's change (platinum, then gold)
/// or a withdrawal's canonical split (crystal, then platinum, then gold), each only when positive.
fn expected_outputs(conversion: u32, amount: u64, change: u64) -> Vec<(Coin, u64)> {
    let split = if conversion == CONVERSION_DEPOSIT {
        vec![(Coin::Platinum, change / 100), (Coin::Gold, change % 100)]
    } else {
        vec![
            (Coin::Crystal, amount / 10_000),
            (Coin::Platinum, amount % 10_000 / 100),
            (Coin::Gold, amount % 100),
        ]
    };
    split
        .into_iter()
        .filter(|(_, quantity)| *quantity > 0)
        .collect()
}

fn check_conversion(value: &BankOperationV1, conversion: u32) -> Result<(), AuditError> {
    check_uuid_v7(&value.backpack_item_instance_id)?;
    let (cap, ledger) = match conversion {
        CONVERSION_DEPOSIT => (BANK_DEPOSIT_MAX, LEDGER_DEPOSIT),
        CONVERSION_WITHDRAW => (BANK_WITHDRAW_MAX, LEDGER_WITHDRAW),
        _ => return Err(AuditError::InvalidInput),
    };
    if !value.recipient_character_id.is_empty() || !(1..=cap).contains(&value.amount) {
        return Err(AuditError::InvalidInput);
    }
    let [line] = value.value_lines.as_slice() else {
        return Err(AuditError::InvalidInput);
    };
    check_value_line(
        line,
        &value.world_id,
        ledger,
        LINE_CLASS_CONVERSION,
        value.amount,
    )?;

    let inputs: Vec<&BankCoinLineV1> = value
        .coin_lines
        .iter()
        .take_while(|line| line.direction == COIN_LINE_INPUT)
        .collect();
    let outputs = &value.coin_lines[inputs.len()..];
    if inputs.len() > BANK_DEPOSIT_INPUTS_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    if (conversion == CONVERSION_DEPOSIT) == inputs.is_empty() {
        return Err(AuditError::InvalidInput);
    }
    let mut worth: u64 = 0;
    let mut previous: Option<(u64, u64)> = None;
    for (index, line) in inputs.iter().enumerate() {
        let before = line.before.as_ref().ok_or(AuditError::InvalidInput)?;
        let after = line.after.as_ref().ok_or(AuditError::InvalidInput)?;
        check_uuid_v7(&before.item_instance_id)?;
        let coin = state_coin(before)?;
        let last = index + 1 == inputs.len();
        let retired = after.lifecycle == ITEM_LIFECYCLE_RETIRED && after.quantity == 0;
        let partial = after.lifecycle == ITEM_LIFECYCLE_LIVE && after.quantity > 0;
        // Plan order: worth ascending, then display order (highest ordinal first); only the
        // last input may keep units.
        let ordered = previous.is_none_or(|(worth, ordinal)| {
            coin.worth() > worth || (coin.worth() == worth && line.placement_ordinal < ordinal)
        });
        if !ordered
            || line.line_class != LINE_CLASS_CONVERSION
            || before.item_instance_id != after.item_instance_id
            || before.world_id != value.world_id
            || after.world_id != value.world_id
            || before.definition != after.definition
            || before.lifecycle != ITEM_LIFECYCLE_LIVE
            || !(1..=COIN_STACK_MAXIMUM).contains(&before.quantity)
            || after.quantity >= before.quantity
            || !(retired || (partial && last))
            || line.placement_ordinal == 0
            || inputs[..index].iter().any(|other| {
                other.before.as_ref().map(|b| &b.item_instance_id) == Some(&before.item_instance_id)
            })
        {
            return Err(AuditError::InvalidInput);
        }
        previous = Some((coin.worth(), line.placement_ordinal));
        worth += u64::from(before.quantity - after.quantity) * coin.worth();
    }
    // A deposit's change is below the last input's worth; a withdrawal has none.
    let change = match previous {
        Some((last_worth, _)) => match worth.checked_sub(value.amount) {
            Some(change) if change < last_worth => change,
            _ => return Err(AuditError::InvalidInput),
        },
        None => 0,
    };
    let expected = expected_outputs(conversion, value.amount, change);
    let ceiling = if conversion == CONVERSION_DEPOSIT {
        BANK_DEPOSIT_CHANGE_MAX
    } else {
        BANK_WITHDRAW_OUTPUTS_MAX
    };
    if outputs.len() > ceiling || outputs.len() != expected.len() {
        return Err(AuditError::InvalidInput);
    }
    let last_input_ordinal = inputs.iter().map(|line| line.placement_ordinal).max();
    let mut previous_ordinal: Option<u64> = None;
    for (line, (coin, quantity)) in outputs.iter().zip(expected) {
        let after = line.after.as_ref().ok_or(AuditError::InvalidInput)?;
        check_uuid_v7(&after.item_instance_id)?;
        let consecutive = match previous_ordinal {
            Some(ordinal) => ordinal.checked_add(1) == Some(line.placement_ordinal),
            None => last_input_ordinal.is_none_or(|ordinal| line.placement_ordinal > ordinal),
        };
        if line.direction != COIN_LINE_OUTPUT
            || line.line_class != LINE_CLASS_CONVERSION
            || line.before.is_some()
            || state_coin(after)? != coin
            || u64::from(after.quantity) != quantity
            || after.lifecycle != ITEM_LIFECYCLE_LIVE
            || after.world_id != value.world_id
            || line.placement_ordinal == 0
            || !consecutive
            || value
                .coin_lines
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
        previous_ordinal = Some(line.placement_ordinal);
    }
    Ok(())
}

fn check_transfer(value: &BankOperationV1) -> Result<(), AuditError> {
    check_uuid_v7(&value.recipient_character_id)?;
    let [out, into] = value.value_lines.as_slice() else {
        return Err(AuditError::InvalidInput);
    };
    if !value.backpack_item_instance_id.is_empty()
        || !value.coin_lines.is_empty()
        || value.recipient_character_id == value.acting_character_id
        || !(1..=BANK_TRANSFER_MAX).contains(&value.amount)
        || out.account_id == into.account_id
        || out.entry_id == into.entry_id
    {
        return Err(AuditError::InvalidInput);
    }
    check_value_line(
        out,
        &value.world_id,
        LEDGER_TRANSFER_OUT,
        LINE_CLASS_TRANSFER,
        value.amount,
    )?;
    check_value_line(
        into,
        &value.world_id,
        LEDGER_TRANSFER_IN,
        LINE_CLASS_TRANSFER,
        value.amount,
    )
}

/// Complete closed bank shape and every registered bound.
pub fn check_bank_operation(value: &BankOperationV1) -> Result<(), AuditError> {
    if value.interpretation_revision != INTERPRETATION_REVISION {
        return Err(AuditError::InvalidInput);
    }
    check_uuid_v7(&value.acting_character_id)?;
    check_uuid_v7(&value.world_id)?;
    check_uuid_v7(&value.channel_id)?;
    if value.runtime_scope_ownership_generation == 0 {
        return Err(AuditError::InvalidInput);
    }
    if value.coin_lines.len() as u64 > BANK_RL01_TOUCHED_ITEM_INSTANCES_MAX
        || value.value_lines.len() as u64 > BANK_RL03_VALUE_LINES_MAX
    {
        return Err(AuditError::CapacityExceeded);
    }
    match value.cause.as_ref().and_then(|cause| cause.cause.as_ref()) {
        Some(BankCauseV1::Conversion(cause)) => {
            check_uuid_v7(&cause.occurrence)?;
            check_conversion(value, cause.kind)
        }
        Some(BankCauseV1::Transfer(cause)) => {
            check_uuid_v7(&cause.occurrence)?;
            check_transfer(value)
        }
        None => Err(AuditError::InvalidInput),
    }
}

/// RL-07 payload gate: size before decode, canonical round trip, closed shape and bounds.
pub fn decode_bank_payload(wire: &[u8]) -> Result<BankOperationV1, AuditError> {
    if wire.len() > BANK_RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = BankOperationV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire {
        return Err(AuditError::InvalidInput);
    }
    check_bank_operation(&value)?;
    Ok(value)
}

/// RL-07 envelope gate: size before decode, canonical round trip, the registered event type 3
/// binding, identity and digest widths, one-event membership, the payload gate, the envelope
/// scope equal to the payload's and no session, command or causation (the cause carries the
/// operation occurrence).
pub fn decode_bank_envelope(wire: &[u8]) -> Result<(EventEnvelopeV1, BankOperationV1), AuditError> {
    if wire.len() > BANK_RL07_ENVELOPE_BYTES_MAX {
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
    check_uuid_v7(&value.event_id)?;
    check_uuid_v7(&membership.transaction_id)?;
    if value.retention_profile_id.len() > ANL_ENVELOPE_STRING_MAX
        || value.server_build_id.is_empty()
        || value.server_build_id.len() > ANL_ENVELOPE_STRING_MAX
        || value.payload_sha256 != Sha256::digest(&value.payload).as_slice()
        || value.envelope_revision != ENVELOPE_REVISION
        || value.event_type_id != EVENT_TYPE_ID
        || value.event_schema_revision != EVENT_SCHEMA_REVISION
        || value.durability_class != DURABLE_AUDIT
        || value.privacy_class != RESTRICTED_PLAYER_LINKED
        || value.retention_profile_id != RETENTION_PROFILE_ID
        || value.occurred_at_unix_ms <= 0
        || membership.ordinal != 1
        || membership.count != 1
    {
        return Err(AuditError::InvalidInput);
    }
    let operation = decode_bank_payload(&value.payload)?;
    if value.world_id.as_deref() != Some(operation.world_id.as_slice())
        || value.channel_id.as_deref() != Some(operation.channel_id.as_slice())
        || value.game_session_id.is_some()
        || value.command_id.is_some()
        || value.connection_generation.is_some()
        || value.causation.is_some()
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((value, operation))
}

/// Envelope identity fixed before the first commit attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: &'a str,
}

/// Build the exact immutable event bytes of one bank operation; re-decoded through both gates
/// before they are returned.
pub fn encode_bank_event(
    identity: BankEventIdentity<'_>,
    operation: BankOperationV1,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    mint::check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    check_bank_operation(&operation)?;
    let (world_id, channel_id) = (operation.world_id.clone(), operation.channel_id.clone());
    let payload = encode_bounded(&operation, BANK_RL07_PAYLOAD_BYTES_MAX)?;
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
    let wire = encode_bounded(&envelope, BANK_RL07_ENVELOPE_BYTES_MAX)?;
    let (decoded, _) = decode_bank_envelope(&wire)?;
    if decoded != envelope {
        return Err(AuditError::InvalidInput);
    }
    Ok(wire)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use serde_json::Value;

    const RESOURCE_REGISTRY: &str =
        include_str!("../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");
    const EVENT_REGISTRY: &str =
        include_str!("../../../../docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json");

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

    fn state(coin: Coin, item: u8, revision: &str, quantity: u32) -> BankItemStateV1 {
        BankItemStateV1 {
            item_instance_id: uuid(item),
            world_id: uuid(1),
            definition: Some(BankItemDefinitionV1 {
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

    fn input(coin: Coin, item: u8, ordinal: u64, before: u32, after: u32) -> BankCoinLineV1 {
        BankCoinLineV1 {
            direction: COIN_LINE_INPUT,
            before: Some(state(coin, item, "r", before)),
            after: Some(state(coin, item, "r", after)),
            placement_ordinal: ordinal,
            line_class: LINE_CLASS_CONVERSION,
        }
    }

    fn output(coin: Coin, item: u8, quantity: u32, ordinal: u64) -> BankCoinLineV1 {
        BankCoinLineV1 {
            direction: COIN_LINE_OUTPUT,
            before: None,
            after: Some(state(coin, item, "r", quantity)),
            placement_ordinal: ordinal,
            line_class: LINE_CLASS_CONVERSION,
        }
    }

    fn value(
        entry: u8,
        account: u8,
        kind: u32,
        class: u32,
        amount: u64,
        before: u64,
    ) -> BankValueLineV1 {
        let after = if matches!(kind, LEDGER_DEPOSIT | LEDGER_TRANSFER_IN) {
            before + amount
        } else {
            before - amount
        };
        BankValueLineV1 {
            entry_id: uuid(entry),
            asset: ASSET_GOLD.into(),
            account_id: uuid(account),
            world_id: uuid(1),
            kind,
            line_class: class,
            amount,
            balance_before: before,
            balance_after: after,
        }
    }

    fn operation(cause: BankCauseV1, amount: u64) -> BankOperationV1 {
        BankOperationV1 {
            interpretation_revision: INTERPRETATION_REVISION,
            cause: Some(BankOperationCauseV1 { cause: Some(cause) }),
            acting_character_id: uuid(41),
            world_id: uuid(1),
            channel_id: uuid(2),
            runtime_scope_ownership_generation: 1,
            amount,
            backpack_item_instance_id: Vec::new(),
            coin_lines: Vec::new(),
            value_lines: Vec::new(),
            recipient_character_id: Vec::new(),
        }
    }

    /// 2,350 deposited from 10 gold and one crystal: 76 platinum and 60 gold of change.
    fn deposit() -> BankOperationV1 {
        let mut value_ = operation(
            BankCauseV1::Conversion(BankConversionCauseV1 {
                kind: CONVERSION_DEPOSIT,
                occurrence: uuid(70),
            }),
            2_350,
        );
        value_.backpack_item_instance_id = uuid(50);
        value_.coin_lines = vec![
            input(Coin::Gold, 10, 2, 10, 0),
            input(Coin::Crystal, 11, 1, 1, 0),
            output(Coin::Platinum, 20, 76, 3),
            output(Coin::Gold, 21, 60, 4),
        ];
        value_.value_lines = vec![value(
            60,
            40,
            LEDGER_DEPOSIT,
            LINE_CLASS_CONVERSION,
            2_350,
            5,
        )];
        value_
    }

    /// 1,009,999 withdrawn as 100 crystal, 99 platinum and 99 gold.
    fn withdraw() -> BankOperationV1 {
        let mut value_ = operation(
            BankCauseV1::Conversion(BankConversionCauseV1 {
                kind: CONVERSION_WITHDRAW,
                occurrence: uuid(70),
            }),
            BANK_WITHDRAW_MAX,
        );
        value_.backpack_item_instance_id = uuid(50);
        value_.coin_lines = vec![
            output(Coin::Crystal, 20, 100, 8),
            output(Coin::Platinum, 21, 99, 9),
            output(Coin::Gold, 22, 99, 10),
        ];
        value_.value_lines = vec![value(
            60,
            40,
            LEDGER_WITHDRAW,
            LINE_CLASS_CONVERSION,
            BANK_WITHDRAW_MAX,
            BANK_WITHDRAW_MAX,
        )];
        value_
    }

    fn transfer() -> BankOperationV1 {
        let mut value_ = operation(
            BankCauseV1::Transfer(BankTransferCauseV1 {
                occurrence: uuid(70),
            }),
            500,
        );
        value_.recipient_character_id = uuid(42);
        value_.value_lines = vec![
            value(60, 40, LEDGER_TRANSFER_OUT, LINE_CLASS_TRANSFER, 500, 800),
            value(61, 43, LEDGER_TRANSFER_IN, LINE_CLASS_TRANSFER, 500, 0),
        ];
        value_
    }

    fn identity() -> BankEventIdentity<'static> {
        BankEventIdentity {
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
                .any(|consumer| consumer
                    .as_str()
                    .unwrap()
                    .starts_with("apps/game-server/src/durability/bank")),
            "{id} names the bank runtime consumer"
        );
        rows[0]["hard_maximum"].as_u64().unwrap()
    }

    #[test]
    fn bank_rows_equal_the_registry() {
        assert_eq!(registered("BANK0-RL-01"), BANK_BALANCE_MAX);
        assert_eq!(registered("BANK0-RL-02-DEPOSIT"), BANK_DEPOSIT_MAX);
        assert_eq!(registered("BANK0-RL-02-WITHDRAW"), BANK_WITHDRAW_MAX);
        assert_eq!(registered("BANK0-RL-02-TRANSFER"), BANK_TRANSFER_MAX);
        assert_eq!(registered("BANK0-RL-03"), BANK_ENTRIES_MAX);
        assert_eq!(
            registered("DUR03-RL-01-BANK"),
            BANK_RL01_TOUCHED_ITEM_INSTANCES_MAX
        );
        assert_eq!(registered("DUR03-RL-03-BANK"), BANK_RL03_VALUE_LINES_MAX);
        assert_eq!(
            registered("DUR03-RL-07-BANK-PAYLOAD-BYTES"),
            BANK_RL07_PAYLOAD_BYTES_MAX as u64
        );
        assert_eq!(
            registered("DUR03-RL-07-BANK-ENVELOPE-BYTES"),
            BANK_RL07_ENVELOPE_BYTES_MAX as u64
        );
        // The deposit's ceiling is 20 stacks of 100 crystal coins; the withdrawal's is the
        // largest canonical split of three stacks.
        assert_eq!(
            BANK_DEPOSIT_MAX,
            BANK_DEPOSIT_INPUTS_MAX as u64 * 100 * Coin::Crystal.worth()
        );
        assert_eq!(BANK_WITHDRAW_MAX, 100 * 10_000 + 99 * 100 + 99);
        // The one-item shapes keep their rows (RL-03 stays 0).
        assert_eq!(mint::RL03_VALUE_LINES_MAX, 0);
    }

    #[test]
    fn event_type_3_is_registered_with_the_economy_ledger_profile() {
        let registry: Value = serde_json::from_str(EVENT_REGISTRY).unwrap();
        let types: Vec<&Value> = registry["event_types"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["id"] == u64::from(EVENT_TYPE_ID))
            .collect();
        assert_eq!(types.len(), 1);
        let row = types[0];
        assert_eq!(row["name"], "BANK_OPERATION");
        assert_eq!(row["retention_profile_id"], RETENTION_PROFILE_ID);
        assert_eq!(
            row["current_schema_revision"],
            u64::from(EVENT_SCHEMA_REVISION)
        );
        assert_eq!(row["payload_message"], "oteryn.events.v2.BankOperationV1");
        let profile = registry["retention_profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|profile| profile["id"] == RETENTION_PROFILE_ID)
            .unwrap();
        assert!(
            profile["finite_retention_duration_or_ceiling"]
                .as_str()
                .unwrap()
                .starts_with("P30D = 2,592,000 elapsed seconds")
        );
        assert_eq!(RETENTION_P30D_MS, 2_592_000 * 1_000);
    }

    #[test]
    fn every_shape_round_trips_and_other_gates_refuse_it() {
        for value_ in [deposit(), withdraw(), transfer()] {
            let wire = encode_bank_event(identity(), value_.clone()).unwrap();
            let (envelope, decoded) = decode_bank_envelope(&wire).unwrap();
            assert_eq!(decoded, value_);
            assert_eq!(envelope.event_type_id, 3);
            assert_eq!(envelope.retention_profile_id, RETENTION_PROFILE_ID);
            assert_eq!(envelope.game_session_id, None);
            // The one-item gate refuses a bank envelope.
            assert!(mint::decode_envelope(&wire).is_err());
        }
    }

    #[test]
    fn every_broken_invariant_is_rejected() {
        let mut cases: Vec<(&str, BankOperationV1)> = Vec::new();
        let mut broken = deposit();
        broken.amount = 2_351;
        broken.value_lines[0] = value(60, 40, LEDGER_DEPOSIT, LINE_CLASS_CONVERSION, 2_351, 5);
        cases.push(("deposit worth other than its credit", broken));
        let mut broken = deposit();
        broken.coin_lines.swap(0, 1);
        cases.push(("plan order broken", broken));
        let mut broken = deposit();
        broken.coin_lines.swap(2, 3);
        cases.push(("gold change before platinum", broken));
        let mut broken = deposit();
        broken.coin_lines[3].placement_ordinal = 5;
        cases.push(("change entries not consecutive", broken));
        let mut broken = deposit();
        broken.coin_lines[2].placement_ordinal = 2;
        broken.coin_lines[3].placement_ordinal = 3;
        cases.push(("change at an input's entry", broken));
        let mut broken = deposit();
        broken.coin_lines[3] = output(Coin::Gold, 20, 60, 4);
        cases.push(("one item twice", broken));
        let mut broken = deposit();
        broken.coin_lines[0].line_class = LINE_CLASS_TRANSFER;
        cases.push(("a coin line of another class", broken));
        let mut broken = deposit();
        broken.value_lines[0].balance_after += 1;
        cases.push(("value line not conserved", broken));
        let mut broken = deposit();
        broken.value_lines[0].kind = LEDGER_WITHDRAW;
        cases.push(("value line of another kind", broken));
        let mut broken = deposit();
        broken.value_lines[0].asset = "platinum".into();
        cases.push(("another asset", broken));
        let mut broken = deposit();
        broken.value_lines.clear();
        cases.push(("no value line", broken));
        let mut broken = deposit();
        broken.backpack_item_instance_id.clear();
        cases.push(("deposit without its backpack", broken));
        let mut broken = deposit();
        broken.recipient_character_id = uuid(42);
        cases.push(("deposit with a recipient", broken));
        let mut broken = deposit();
        broken.coin_lines[0].after.as_mut().unwrap().world_id = uuid(3);
        cases.push(("World changed", broken));
        let mut broken = withdraw();
        broken.coin_lines[2] = output(Coin::Gold, 22, 98, 10);
        cases.push(("withdrawal worth other than its debit", broken));
        let mut broken = withdraw();
        broken.coin_lines.swap(0, 1);
        cases.push(("withdrawal split out of order", broken));
        let mut broken = withdraw();
        broken.amount = BANK_WITHDRAW_MAX + 1;
        cases.push(("withdrawal above its row", broken));
        let mut broken = withdraw();
        broken.coin_lines.insert(0, input(Coin::Gold, 9, 1, 5, 0));
        cases.push(("withdrawal with an input", broken));
        let mut broken = transfer();
        broken.value_lines[1].account_id = uuid(40);
        cases.push(("transfer within one Account", broken));
        let mut broken = transfer();
        broken.value_lines.swap(0, 1);
        cases.push(("transfer lines out of order", broken));
        let mut broken = transfer();
        broken.value_lines[1].amount = 499;
        broken.value_lines[1].balance_after = 499;
        cases.push(("transfer amounts differ", broken));
        let mut broken = transfer();
        broken.value_lines[0].line_class = LINE_CLASS_CONVERSION;
        cases.push(("transfer line of another class", broken));
        let mut broken = transfer();
        broken.recipient_character_id = uuid(41);
        cases.push(("transfer to oneself", broken));
        let mut broken = transfer();
        broken.coin_lines = vec![output(Coin::Gold, 22, 1, 1)];
        cases.push(("transfer with a coin line", broken));
        let mut broken = transfer();
        broken.value_lines[0].balance_before = 400;
        cases.push(("transfer overdraws", broken));
        let mut broken = transfer();
        broken.cause = None;
        cases.push(("no cause", broken));
        let mut broken = transfer();
        broken.interpretation_revision = 2;
        cases.push(("unknown interpretation", broken));
        for (case, value_) in cases {
            assert!(check_bank_operation(&value_).is_err(), "{case}");
        }
    }

    fn worst_case() -> BankOperationV1 {
        let revision = "r".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX);
        let crystal = |item: u8, ordinal: u64| BankCoinLineV1 {
            direction: COIN_LINE_INPUT,
            before: Some(state(Coin::Crystal, item, &revision, 100)),
            after: Some(state(Coin::Crystal, item, &revision, 0)),
            placement_ordinal: ordinal,
            line_class: LINE_CLASS_CONVERSION,
        };
        let mut lines: Vec<BankCoinLineV1> = (0..BANK_DEPOSIT_INPUTS_MAX)
            .map(|n| crystal(100 + n as u8, u64::MAX - 2 - n as u64))
            .collect();
        for (n, (coin, quantity)) in [(Coin::Platinum, 99), (Coin::Gold, 99)]
            .into_iter()
            .enumerate()
        {
            lines.push(BankCoinLineV1 {
                direction: COIN_LINE_OUTPUT,
                before: None,
                after: Some(state(coin, 200 + n as u8, &revision, quantity)),
                placement_ordinal: u64::MAX - 1 + n as u64,
                line_class: LINE_CLASS_CONVERSION,
            });
        }
        let amount = BANK_DEPOSIT_MAX - 9_999;
        let mut value_ = operation(
            BankCauseV1::Conversion(BankConversionCauseV1 {
                kind: CONVERSION_DEPOSIT,
                occurrence: uuid(70),
            }),
            amount,
        );
        value_.runtime_scope_ownership_generation = u64::MAX;
        value_.backpack_item_instance_id = uuid(50);
        value_.coin_lines = lines;
        value_.value_lines = vec![value(
            60,
            40,
            LEDGER_DEPOSIT,
            LINE_CLASS_CONVERSION,
            amount,
            BANK_BALANCE_MAX - amount,
        )];
        value_
    }

    // Within the ANL-01 ceilings (payload 196,608 B, envelope 262,144 B).
    const _: () = assert!(BANK_RL07_PAYLOAD_BYTES_MAX <= 196_608);
    const _: () = assert!(BANK_RL07_ENVELOPE_BYTES_MAX <= 262_144);

    #[test]
    fn worst_case_payload_and_envelope_are_the_registered_rows() {
        // A 20-input deposit of whole crystal stacks with the largest change (99 platinum, 99
        // gold), 512-byte revisions, maximal integers and a balance ending at its maximum: the
        // exact encoded worst case of this schema. A withdrawal or a transfer is smaller.
        let value_ = worst_case();
        assert_eq!(value_.encoded_len(), BANK_RL07_PAYLOAD_BYTES_MAX);
        let identity = BankEventIdentity {
            occurred_at_unix_ms: i64::MAX,
            server_build_id: &"b".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX),
            ..identity()
        };
        let wire = encode_bank_event(identity, value_.clone()).unwrap();
        assert_eq!(wire.len(), BANK_RL07_ENVELOPE_BYTES_MAX);
        assert!(withdraw().encoded_len() < BANK_RL07_PAYLOAD_BYTES_MAX);
        assert!(transfer().encoded_len() < BANK_RL07_PAYLOAD_BYTES_MAX);
        // max + 1: a 21st input is rejected before encoding.
        let mut over = worst_case();
        let extra = over.coin_lines[0].clone();
        over.coin_lines.insert(0, extra);
        assert_eq!(
            encode_bank_event(identity, over),
            Err(AuditError::CapacityExceeded)
        );
        // One byte over either ceiling is refused before decoding.
        let mut long = wire.clone();
        long.push(0);
        assert_eq!(
            decode_bank_envelope(&long),
            Err(AuditError::CapacityExceeded)
        );
        assert_eq!(
            decode_bank_payload(&vec![0; BANK_RL07_PAYLOAD_BYTES_MAX + 1]),
            Err(AuditError::CapacityExceeded)
        );
    }
}
