//! Registered DUR-03 NPC trade durable-audit encoding (NPC-TRADE-1; NPC-0 §5.5,
//! `reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md`): event type 2
//! (`oteryn.events.v1.OneItemTransactionV1`, operation `npc_trade`, oneof tag 9) in the normative
//! ANL-01 `EventEnvelope`.
//!
//! One event per trade transaction carries the closed `NpcTradeCause {npc, offer, side,
//! occurrence}`, the CommandRef it is bound to, the price and every line in order:
//! - BUY: the coin BURN lines of the gold fee plan (`FeeBurnCause::NpcTrade`; worth ascending,
//!   then display order; only the last keeps units), the change MINT (platinum, then gold) and the
//!   bought item's MINT, each MINT a fresh stack in a new direct backpack entry;
//! - SELL: one BURN of the sold item (part of a stack or the whole item), then the coin MINT
//!   (crystal, platinum, gold, each only when positive) in new entries.
//!
//! The shape has its own `DUR03-RL-*-NPC-TRADE-*` rows (BUY 23 participants / 66 work units, SELL
//! 4 / 9); every bound is checked before encode allocation and oversize input is rejected, never
//! truncated.

use super::item_mint_audit::{
    self as mint, AuditError, EventEnvelopeV1, ITEM_LIFECYCLE_LIVE, OneItemOperationV1,
    OneItemStateV1, OneItemTransactionV1, TransactionEventRefV1, TransactionResourceUsage,
    Type2EventTuple, check_content_key, check_definition, check_uuid_v7, encode_bounded,
};
use super::item_transfer_audit::{ITEM_LIFECYCLE_RETIRED, OneItemCommandRefV1};
use crate::domain::currency::{COIN_DEFINITION_FAMILY, COIN_STACK_MAXIMUM, Coin, FEE_INPUTS_MAX};
use prost::Message;
use sha2::{Digest, Sha256};

/// BUY rows (NPC-0 §5.5): 20 coin BURN lines, 2 change MINTs and the item MINT.
pub const BUY_RL01_TOUCHED_ITEM_INSTANCES_MAX: u64 = 23;
pub const BUY_RL02_LOCATION_CUSTODY_LINES_MAX: u64 = 23;
pub const BUY_RL06_PARTICIPANTS_MAX: u64 = 23;
pub const BUY_RL06_EFFECT_WORK_UNITS_MAX: u64 = 66;
/// SELL rows (NPC-0 §5.5): the item BURN and 3 coin MINTs.
pub const SELL_RL01_TOUCHED_ITEM_INSTANCES_MAX: u64 = 4;
pub const SELL_RL02_LOCATION_CUSTODY_LINES_MAX: u64 = 4;
pub const SELL_RL06_PARTICIPANTS_MAX: u64 = 4;
pub const SELL_RL06_EFFECT_WORK_UNITS_MAX: u64 = 9;
pub const NPC_TRADE_RL07_EVENTS_MAX: u64 = 1;
/// Measured worst case of this schema (the `worst_case_*` test), D50 method: a BUY of 20 whole
/// coin burns, the largest change and an item MINT, every key and revision at 512 bytes.
pub const NPC_TRADE_RL07_PAYLOAD_BYTES_MAX: usize = 29_449;
pub const NPC_TRADE_RL07_ENVELOPE_BYTES_MAX: usize = 29_763;

/// The largest BUY price: what 20 stacks of 100 crystal coins hold (NPC-0 §5.3).
pub const NPC_TRADE_BUY_GOLD_UNITS_MAX: u64 = 20_000_000;
/// The largest SELL value: 100 crystal, 99 platinum and 99 gold coins (NPC-0 §5.4).
pub const NPC_TRADE_SELL_GOLD_UNITS_MAX: u64 = 1_009_999;
/// Units of one trade: at most one stack (D82).
pub const NPC_TRADE_QUANTITY_MAX: u32 = 100;

pub const SIDE_BUY: u32 = 1;
pub const SIDE_SELL: u32 = 2;
pub const LINE_COIN_BURN: u32 = 1;
pub const LINE_COIN_MINT: u32 = 2;
pub const LINE_ITEM_BURN: u32 = 3;
pub const LINE_ITEM_MINT: u32 = 4;

/// `NpcTradeCause {npc, offer, side, occurrence}` (NPC-0 §5.5). The offer is its index in the
/// NPC's catalogue at the bound catalogue revision.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemNpcTradeCauseV1 {
    #[prost(string, tag = "1")]
    pub npc_key: String,
    #[prost(uint64, tag = "2")]
    pub catalogue_revision: u64,
    #[prost(uint32, tag = "3")]
    pub offer_index: u32,
    /// 1 = BUY, 2 = SELL.
    #[prost(uint32, tag = "4")]
    pub side: u32,
    #[prost(bytes = "vec", tag = "5")]
    pub occurrence: Vec<u8>,
}

/// One line. A BURN has both states (a whole burn ends `RETIRED` with no location); a MINT has
/// only `after`, a fresh live stack in a new direct backpack entry.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemNpcTradeLineV1 {
    /// 1 = coin BURN, 2 = coin MINT, 3 = item BURN, 4 = item MINT.
    #[prost(uint32, tag = "1")]
    pub kind: u32,
    #[prost(message, optional, tag = "2")]
    pub before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "3")]
    pub after: Option<OneItemStateV1>,
    /// The entry before a BURN (and after a partial one), or the MINT's new entry.
    #[prost(uint64, tag = "4")]
    pub placement_ordinal: u64,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemNpcTradeV1 {
    #[prost(message, optional, tag = "1")]
    pub cause: Option<OneItemNpcTradeCauseV1>,
    /// The command 8 the occurrence is bound to.
    #[prost(message, optional, tag = "2")]
    pub command: Option<OneItemCommandRefV1>,
    #[prost(bytes = "vec", tag = "3")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub channel_id: Vec<u8>,
    #[prost(uint64, tag = "6")]
    pub runtime_scope_ownership_generation: u64,
    /// The equipped main backpack: the parent of every line's entry.
    #[prost(bytes = "vec", tag = "7")]
    pub backpack_item_instance_id: Vec<u8>,
    /// The offer quantity (1..=100) and the server's unit price at the bound revision.
    #[prost(uint32, tag = "8")]
    pub quantity: u32,
    #[prost(uint64, tag = "9")]
    pub unit_price: u64,
    /// `unit_price x quantity`.
    #[prost(uint64, tag = "10")]
    pub total_gold_units: u64,
    /// Conservation summary: BUY `burned - minted = total`, SELL `burned = 0, minted = total`.
    #[prost(uint64, tag = "11")]
    pub burned_gold_units: u64,
    #[prost(uint64, tag = "12")]
    pub minted_gold_units: u64,
    #[prost(message, repeated, tag = "13")]
    pub lines: Vec<OneItemNpcTradeLineV1>,
}

/// Resource usage of one trade: each line is a participant; a whole burn adds its location
/// removal and its retirement, a partial burn its quantity change, a MINT its new entry.
pub fn npc_trade_usage(value: &OneItemNpcTradeV1) -> TransactionResourceUsage {
    let burn = |line: &&OneItemNpcTradeLineV1| matches!(line.kind, LINE_COIN_BURN | LINE_ITEM_BURN);
    let burns = value.lines.iter().filter(burn).count() as u64;
    let whole = value
        .lines
        .iter()
        .filter(burn)
        .filter(|line| line.after.as_ref().is_some_and(|after| after.quantity == 0))
        .count() as u64;
    let count = value.lines.len() as u64;
    let minted = count - burns;
    TransactionResourceUsage {
        touched_item_instances: count,
        location_custody_lines: whole + minted,
        value_lines: 0,
        transform_lines: 0,
        container_expansion: 0,
        participants: count,
        effect_work_units: 3 * whole + 2 * (burns - whole) + 2 * minted,
        events: 1,
    }
}

fn check_npc_trade_usage(side: u32, usage: &TransactionResourceUsage) -> Result<(), AuditError> {
    let (touched, custody, participants, work) = if side == SIDE_BUY {
        (
            BUY_RL01_TOUCHED_ITEM_INSTANCES_MAX,
            BUY_RL02_LOCATION_CUSTODY_LINES_MAX,
            BUY_RL06_PARTICIPANTS_MAX,
            BUY_RL06_EFFECT_WORK_UNITS_MAX,
        )
    } else {
        (
            SELL_RL01_TOUCHED_ITEM_INSTANCES_MAX,
            SELL_RL02_LOCATION_CUSTODY_LINES_MAX,
            SELL_RL06_PARTICIPANTS_MAX,
            SELL_RL06_EFFECT_WORK_UNITS_MAX,
        )
    };
    if usage.value_lines != 0 || usage.transform_lines != 0 || usage.container_expansion != 0 {
        return Err(AuditError::InvalidInput);
    }
    if usage.touched_item_instances > touched
        || usage.location_custody_lines > custody
        || usage.participants > participants
        || usage.effect_work_units > work
        || usage.events > NPC_TRADE_RL07_EVENTS_MAX
    {
        return Err(AuditError::CapacityExceeded);
    }
    Ok(())
}

/// The coins a SELL of `value` gold units mints: crystal, platinum, gold, each only when
/// positive (NPC-0 §5.4). `None` when a stack would exceed 100.
pub fn sell_coins(value: u64) -> Option<Vec<(Coin, u32)>> {
    let crystal = value / Coin::Crystal.worth();
    if crystal > u64::from(COIN_STACK_MAXIMUM) {
        return None;
    }
    Some(
        [
            (Coin::Crystal, crystal),
            (Coin::Platinum, value % Coin::Crystal.worth() / Coin::Platinum.worth()),
            (Coin::Gold, value % Coin::Platinum.worth()),
        ]
        .into_iter()
        .filter(|(_, quantity)| *quantity > 0)
        .map(|(coin, quantity)| (coin, u32::try_from(quantity).unwrap_or(u32::MAX)))
        .collect(),
    )
}

/// The change a BUY mints: `change / 100` platinum, then `change % 100` gold, each only when
/// positive (D175).
pub fn buy_change_coins(change: u64) -> Vec<(Coin, u32)> {
    [(Coin::Platinum, change / 100), (Coin::Gold, change % 100)]
        .into_iter()
        .filter(|(_, quantity)| *quantity > 0)
        .map(|(coin, quantity)| (coin, u32::try_from(quantity).unwrap_or(u32::MAX)))
        .collect()
}

/// The coin of a coin line's state: one of the closed worth table's definitions.
fn state_coin(state: &OneItemStateV1) -> Result<Coin, AuditError> {
    let definition = state.definition.as_ref().ok_or(AuditError::InvalidInput)?;
    match Coin::from_production_key(&definition.production_key) {
        Some(coin) if definition.family == COIN_DEFINITION_FAMILY => Ok(coin),
        _ => Err(AuditError::InvalidInput),
    }
}

/// A traded item is never a coin.
fn is_coin(state: &OneItemStateV1) -> bool {
    state.definition.as_ref().is_some_and(|definition| {
        definition.family == COIN_DEFINITION_FAMILY
            && Coin::from_production_key(&definition.production_key).is_some()
    })
}

fn check_state(state: &OneItemStateV1, world_id: &[u8]) -> Result<(), AuditError> {
    check_uuid_v7(&state.item_instance_id)?;
    check_definition(state.definition.as_ref())?;
    if state.world_id != world_id || state.quantity > COIN_STACK_MAXIMUM {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

/// A BURN line: the same live item before, fewer units after, retired exactly when none is
/// left. Returns the burned units.
fn check_burn(line: &OneItemNpcTradeLineV1, world_id: &[u8]) -> Result<u32, AuditError> {
    let before = line.before.as_ref().ok_or(AuditError::InvalidInput)?;
    let after = line.after.as_ref().ok_or(AuditError::InvalidInput)?;
    check_state(before, world_id)?;
    check_state(after, world_id)?;
    let retired = after.lifecycle == ITEM_LIFECYCLE_RETIRED && after.quantity == 0;
    let partial = after.lifecycle == ITEM_LIFECYCLE_LIVE && after.quantity > 0;
    if before.item_instance_id != after.item_instance_id
        || before.definition != after.definition
        || before.lifecycle != ITEM_LIFECYCLE_LIVE
        || before.quantity == 0
        || after.quantity >= before.quantity
        || !(retired || partial)
        || line.placement_ordinal == 0
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(before.quantity - after.quantity)
}

/// A MINT line: no before state, a fresh live stack.
fn check_mint(line: &OneItemNpcTradeLineV1, world_id: &[u8]) -> Result<u32, AuditError> {
    let after = line.after.as_ref().ok_or(AuditError::InvalidInput)?;
    check_state(after, world_id)?;
    if line.before.is_some()
        || after.lifecycle != ITEM_LIFECYCLE_LIVE
        || after.quantity == 0
        || line.placement_ordinal == 0
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(after.quantity)
}

/// The item of a line, before for a BURN and after for a MINT.
fn line_item(line: &OneItemNpcTradeLineV1) -> Option<&[u8]> {
    line.before
        .as_ref()
        .or(line.after.as_ref())
        .map(|state| state.item_instance_id.as_slice())
}

/// Complete closed trade shape, conservation and every registered bound.
pub fn check_npc_trade(value: &OneItemNpcTradeV1) -> Result<(), AuditError> {
    let cause = value.cause.as_ref().ok_or(AuditError::InvalidInput)?;
    check_content_key(&cause.npc_key)?;
    check_uuid_v7(&cause.occurrence)?;
    let command = value.command.as_ref().ok_or(AuditError::InvalidInput)?;
    check_uuid_v7(&command.game_session_id)?;
    check_uuid_v7(&value.character_id)?;
    check_uuid_v7(&value.world_id)?;
    check_uuid_v7(&value.channel_id)?;
    check_uuid_v7(&value.backpack_item_instance_id)?;
    let side = cause.side;
    let (burn_kind, mint_kinds, total_max) = match side {
        SIDE_BUY => (LINE_COIN_BURN, [LINE_COIN_MINT, LINE_ITEM_MINT], NPC_TRADE_BUY_GOLD_UNITS_MAX),
        SIDE_SELL => (LINE_ITEM_BURN, [LINE_COIN_MINT, LINE_COIN_MINT], NPC_TRADE_SELL_GOLD_UNITS_MAX),
        _ => return Err(AuditError::InvalidInput),
    };
    if command.command_id == 0
        || value.runtime_scope_ownership_generation == 0
        || !(1..=NPC_TRADE_QUANTITY_MAX).contains(&value.quantity)
        || value.unit_price == 0
        || value.unit_price.checked_mul(u64::from(value.quantity)) != Some(value.total_gold_units)
        || value.total_gold_units > total_max
    {
        return Err(AuditError::InvalidInput);
    }
    let burns = value
        .lines
        .iter()
        .take_while(|line| line.kind == burn_kind)
        .count();
    let (burn_lines, mint_lines) = value.lines.split_at(burns);
    if burns == 0
        || (side == SIDE_SELL && burns != 1)
        || burns > FEE_INPUTS_MAX
        || mint_lines
            .iter()
            .any(|line| !mint_kinds.contains(&line.kind))
    {
        return Err(if burns > FEE_INPUTS_MAX {
            AuditError::CapacityExceeded
        } else {
            AuditError::InvalidInput
        });
    }

    // BURN lines. BUY: the fee plan's coin order, only the last keeps units. SELL: one item.
    let mut burned: u64 = 0;
    let mut item_units: Option<u32> = None;
    let mut item_definition = None;
    let mut previous: Option<(u64, u64)> = None;
    for (index, line) in burn_lines.iter().enumerate() {
        let units = check_burn(line, &value.world_id)?;
        let before = line.before.as_ref().ok_or(AuditError::InvalidInput)?;
        if side == SIDE_BUY {
            let coin = state_coin(before)?;
            let last = index + 1 == burn_lines.len();
            let ordered = previous.is_none_or(|(worth, ordinal)| {
                coin.worth() > worth || (coin.worth() == worth && line.placement_ordinal < ordinal)
            });
            let whole = line.after.as_ref().is_some_and(|after| after.quantity == 0);
            if !ordered || !(whole || last) {
                return Err(AuditError::InvalidInput);
            }
            previous = Some((coin.worth(), line.placement_ordinal));
            burned += u64::from(units) * coin.worth();
        } else {
            if is_coin(before) {
                return Err(AuditError::InvalidInput);
            }
            item_units = Some(units);
            item_definition = before.definition.clone();
        }
    }

    // MINT lines: BUY's change then the item; SELL's coins. Consecutive new entries after every
    // BURN line's entry.
    let mut minted: u64 = 0;
    let mut coins = Vec::new();
    let last_burn_ordinal = burn_lines.iter().map(|line| line.placement_ordinal).max();
    let mut previous_ordinal: Option<u64> = None;
    for (index, line) in mint_lines.iter().enumerate() {
        let units = check_mint(line, &value.world_id)?;
        let after = line.after.as_ref().ok_or(AuditError::InvalidInput)?;
        let consecutive = match previous_ordinal {
            Some(ordinal) => ordinal.checked_add(1) == Some(line.placement_ordinal),
            None => last_burn_ordinal.is_none_or(|ordinal| line.placement_ordinal > ordinal),
        };
        if !consecutive {
            return Err(AuditError::InvalidInput);
        }
        previous_ordinal = Some(line.placement_ordinal);
        if line.kind == LINE_ITEM_MINT {
            // The bought item is the last line, and never a coin.
            if index + 1 != mint_lines.len() || is_coin(after) {
                return Err(AuditError::InvalidInput);
            }
            item_units = Some(units);
            item_definition = after.definition.clone();
        } else {
            let coin = state_coin(after)?;
            minted += u64::from(units) * coin.worth();
            coins.push((coin, units));
        }
    }
    let Some(units) = item_units else {
        return Err(AuditError::InvalidInput);
    };
    if item_definition.is_none() || units == 0 {
        return Err(AuditError::InvalidInput);
    }

    // Conservation and the exact coin outputs.
    let expected = if side == SIDE_BUY {
        // The change is below the last burned coin's worth: no unit burned beyond the plan.
        let change_bound = previous.map_or(1, |(worth, _)| worth);
        let change = burned.checked_sub(value.total_gold_units);
        if change.is_none_or(|change| change >= change_bound) {
            return Err(AuditError::InvalidInput);
        }
        buy_change_coins(change.unwrap_or(0))
    } else {
        sell_coins(value.total_gold_units).ok_or(AuditError::InvalidInput)?
    };
    let sell_burned = if side == SIDE_SELL { 0 } else { burned };
    if value.burned_gold_units != sell_burned
        || value.minted_gold_units != minted
        || coins != expected
        || (side == SIDE_SELL && minted != value.total_gold_units)
    {
        return Err(AuditError::InvalidInput);
    }
    // Every line names a distinct item.
    for (index, line) in value.lines.iter().enumerate() {
        if value.lines[..index]
            .iter()
            .any(|other| line_item(other) == line_item(line))
        {
            return Err(AuditError::InvalidInput);
        }
    }
    check_npc_trade_usage(side, &npc_trade_usage(value))
}

/// RL-07 payload gate for the trade shape: size before decode, canonical round trip, closed
/// shape and per-field bounds.
pub fn decode_npc_trade_payload(wire: &[u8]) -> Result<OneItemNpcTradeV1, AuditError> {
    if wire.len() > NPC_TRADE_RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = OneItemTransactionV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire
        || value.interpretation_revision != mint::INTERPRETATION_REVISION
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::NpcTrade(trade)) = value.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_npc_trade(&trade)?;
    Ok(trade)
}

/// RL-07 envelope gate for the trade shape: the shared envelope gate under the trade envelope
/// row (either admitted tuple), the payload gate, the envelope scope equal to the payload's and
/// no session, command or causation (the payload carries the CommandRef and the occurrence).
pub fn decode_npc_trade_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, OneItemNpcTradeV1), AuditError> {
    let value = mint::decode_common_envelope_within(wire, NPC_TRADE_RL07_ENVELOPE_BYTES_MAX)?;
    let trade = decode_npc_trade_payload(&value.payload)?;
    Type2EventTuple::of(value.event_schema_revision, &value.retention_profile_id)
        .ok_or(AuditError::InvalidInput)?;
    if value.world_id.as_deref() != Some(trade.world_id.as_slice())
        || value.channel_id.as_deref() != Some(trade.channel_id.as_slice())
        || value.game_session_id.is_some()
        || value.command_id.is_some()
        || value.connection_generation.is_some()
        || value.causation.is_some()
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((value, trade))
}

/// Envelope identity fixed before the first commit attempt (D177).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcTradeEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: &'a str,
}

/// Build the exact immutable event bytes of one trade under `tuple`; re-decoded through both
/// gates before they are returned.
pub fn encode_npc_trade_event(
    identity: NpcTradeEventIdentity<'_>,
    trade: OneItemNpcTradeV1,
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    mint::check_technical_text(identity.server_build_id)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    check_npc_trade(&trade)?;
    let (world_id, channel_id) = (trade.world_id.clone(), trade.channel_id.clone());
    let payload = encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::NpcTrade(trade)),
        },
        NPC_TRADE_RL07_PAYLOAD_BYTES_MAX,
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
    let wire = encode_bounded(&envelope, NPC_TRADE_RL07_ENVELOPE_BYTES_MAX)?;
    let (decoded, _) = decode_npc_trade_envelope(&wire)?;
    if decoded != envelope {
        return Err(AuditError::InvalidInput);
    }
    Ok(wire)
}

#[cfg(test)]
#[path = "npc_trade_audit_tests.rs"]
mod tests;
