//! NPC-TRADE-SELLALL-1 shop look and sell-all over an open NPC trade window.
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/npc_shop_v1.proto`. Capability 81 `NPC_SHOP_V1`
//! requires capability 3 `NPC_SERVICE_V1` and gates command type 97 `NPC_SHOP`. The intent names
//! one offer of the window the server projected, exactly as `NpcTradeIntentV1` does, and one
//! action: LOOK answers the offer item's server description and changes no state; SELL_ALL sells
//! every carried stack of the offer item under the sell rules of command 8, in one trade.
//!
//! Decoding is strict and encoding refuses the same values: a zero or unknown enum, a zero NPC
//! generation, an offer index at or above `MAX_OFFERS`, unknown or repeated fields, and any byte
//! size over its bound fail closed. A result carries the look text or the sold count and gold
//! only with `OK`, and never both.

use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes,
    read_uint32, read_varint, set_once,
};
use crate::item_look::MAX_ITEM_LOOK_TEXT_BYTES;
pub use crate::npc_service::NpcServiceWireError as NpcShopWireError;
use crate::npc_service::{
    MAX_NPC_INTENT_RESULT_BYTES, NpcIntentDisposition, NpcTradeSide, check_offer_index, check_size,
    decode_npc, encode_npc, malformed, read_fields,
};
use crate::world_spatial_entities::EntityRef;

/// Registered capability `NPC_SHOP_V1`: command type 97, no state domain; requires capability 3.
pub const CAPABILITY_NPC_SHOP_V1: u32 = 81;
pub const COMMAND_TYPE_NPC_SHOP: u32 = 97;
/// SELLALL1-RL-01: the item stacks one sell-all burns (the backpack entry ceiling).
pub const MAX_SELL_ALL_STACKS: usize = 20;
/// SELLALL1-RL-02: the units one sell-all sells (20 stacks of the coin stack maximum).
pub const MAX_SELL_ALL_UNITS: u32 = 2_000;
/// SELLALL1-RL-03: the gold one sell-all pays, the most three coin stacks hold (NPC-0 §5.4).
pub const MAX_SELL_ALL_GOLD_UNITS: u64 = 1_009_999;
/// The NPC field (2 + 29), revision (11), index (3), side (2), action (2) and price (6).
pub const MAX_NPC_SHOP_INTENT_BYTES: usize = 2 + 29 + 11 + 3 + 2 + 2 + 6;
/// SELLALL1-RL-04: the disposition (2), the look text (3 + 512), units (3) and gold (4).
pub const MAX_NPC_SHOP_RESULT_BYTES: usize =
    MAX_NPC_INTENT_RESULT_BYTES + 3 + MAX_ITEM_LOOK_TEXT_BYTES + 3 + 4;

/// `NpcShopAction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcShopAction {
    Look = 1,
    SellAll = 2,
}

impl NpcShopAction {
    fn from_wire(value: u32) -> WireResult<Self> {
        match value {
            1 => Ok(Self::Look),
            2 => Ok(Self::SellAll),
            _ => malformed(),
        }
    }
}

/// `NpcShopIntentV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcShopIntent {
    pub npc_actor: EntityRef,
    pub catalogue_revision: u64,
    /// Below `MAX_OFFERS`.
    pub offer_index: u32,
    pub side: NpcTradeSide,
    pub action: NpcShopAction,
    pub expected_unit_price: u32,
}

/// `NpcShopResultV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcShopResult {
    pub disposition: NpcIntentDisposition,
    /// The look text of an `OK` LOOK; empty otherwise.
    pub look_text: String,
    /// The units an `OK` SELL_ALL sold; zero otherwise.
    pub sold_units: u32,
    /// The gold an `OK` SELL_ALL paid, `sold_units` times the unit price; zero otherwise.
    pub total_gold_units: u64,
}

impl NpcShopResult {
    /// A refusal: the disposition alone.
    pub fn refused(disposition: NpcIntentDisposition) -> Self {
        Self {
            disposition,
            look_text: String::new(),
            sold_units: 0,
            total_gold_units: 0,
        }
    }
}

fn check_intent(intent: &NpcShopIntent) -> WireResult<()> {
    if intent.npc_actor.generation == 0 {
        return malformed();
    }
    check_offer_index(intent.offer_index)
}

/// Encodes an `NpcShopIntentV1` (the client side).
pub fn encode_npc_shop_intent(intent: &NpcShopIntent) -> WireResult<Vec<u8>> {
    check_intent(intent)?;
    let mut output = Vec::with_capacity(MAX_NPC_SHOP_INTENT_BYTES);
    encode_npc(&mut output, 1, &intent.npc_actor)?;
    push_nonzero_varint_field(&mut output, 2, intent.catalogue_revision);
    push_nonzero_varint_field(&mut output, 3, u64::from(intent.offer_index));
    push_varint_field(&mut output, 4, intent.side as u64);
    push_varint_field(&mut output, 5, intent.action as u64);
    push_nonzero_varint_field(&mut output, 6, u64::from(intent.expected_unit_price));
    Ok(output)
}

/// Decodes an `NpcShopIntentV1`; the server answers `REJECTED` to any error.
pub fn decode_npc_shop_intent(payload: &[u8]) -> WireResult<NpcShopIntent> {
    check_size(payload, MAX_NPC_SHOP_INTENT_BYTES)?;
    let (mut npc, mut revision, mut index) = (None, None, None);
    let (mut side, mut action, mut price) = (None, None, None);
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 2) => set_once(&mut npc, decode_npc(read_bytes(input, cursor)?)?),
            (2, 0) => set_once(&mut revision, read_varint(input, cursor)?),
            (3, 0) => set_once(&mut index, read_uint32(input, cursor)?),
            (4, 0) => set_once(
                &mut side,
                NpcTradeSide::from_wire(read_uint32(input, cursor)?)?,
            ),
            (5, 0) => set_once(
                &mut action,
                NpcShopAction::from_wire(read_uint32(input, cursor)?)?,
            ),
            (6, 0) => set_once(&mut price, read_uint32(input, cursor)?),
            _ => malformed(),
        }
    })?;
    let intent = NpcShopIntent {
        npc_actor: npc.ok_or(NpcShopWireError::Malformed)?,
        catalogue_revision: revision.unwrap_or(0),
        offer_index: index.unwrap_or(0),
        side: side.ok_or(NpcShopWireError::Malformed)?,
        action: action.ok_or(NpcShopWireError::Malformed)?,
        expected_unit_price: price.unwrap_or(0),
    };
    check_intent(&intent)?;
    Ok(intent)
}

fn check_result(result: &NpcShopResult) -> WireResult<()> {
    if result.look_text.len() > MAX_ITEM_LOOK_TEXT_BYTES
        || result.sold_units > MAX_SELL_ALL_UNITS
        || result.total_gold_units > MAX_SELL_ALL_GOLD_UNITS
    {
        return Err(NpcShopWireError::LimitExceeded);
    }
    let look = !result.look_text.is_empty();
    let sold = result.sold_units != 0;
    let valid = (sold == (result.total_gold_units != 0))
        && match result.disposition {
            NpcIntentDisposition::Ok => look != sold,
            _ => !look && !sold,
        };
    if !valid {
        return malformed();
    }
    Ok(())
}

/// Encodes an `NpcShopResultV1` (the server side).
pub fn encode_npc_shop_result(result: &NpcShopResult) -> WireResult<Vec<u8>> {
    check_result(result)?;
    let mut output = Vec::with_capacity(MAX_NPC_SHOP_RESULT_BYTES);
    push_varint_field(&mut output, 1, result.disposition as u64);
    if !result.look_text.is_empty() {
        push_message_field(&mut output, 2, result.look_text.as_bytes());
    }
    push_nonzero_varint_field(&mut output, 3, u64::from(result.sold_units));
    push_nonzero_varint_field(&mut output, 4, result.total_gold_units);
    Ok(output)
}

/// Decodes an `NpcShopResultV1` (the client side).
pub fn decode_npc_shop_result(payload: &[u8]) -> WireResult<NpcShopResult> {
    check_size(payload, MAX_NPC_SHOP_RESULT_BYTES)?;
    let (mut disposition, mut text, mut units, mut gold) = (None, None, None, None);
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 0) => {
                let value = read_uint32(input, cursor)?;
                let found = NpcIntentDisposition::ALL
                    .into_iter()
                    .find(|disposition| *disposition as u32 == value)
                    .ok_or(NpcShopWireError::Malformed)?;
                set_once(&mut disposition, found)
            }
            (2, 2) => {
                let bytes = read_bytes(input, cursor)?;
                if bytes.len() > MAX_ITEM_LOOK_TEXT_BYTES {
                    return Err(NpcShopWireError::LimitExceeded);
                }
                let text_value =
                    std::str::from_utf8(bytes).map_err(|_| NpcShopWireError::Malformed)?;
                set_once(&mut text, text_value.to_owned())
            }
            (3, 0) => set_once(&mut units, read_uint32(input, cursor)?),
            (4, 0) => set_once(&mut gold, read_varint(input, cursor)?),
            _ => malformed(),
        }
    })?;
    let result = NpcShopResult {
        disposition: disposition.ok_or(NpcShopWireError::Malformed)?,
        look_text: text.unwrap_or_default(),
        sold_units: units.unwrap_or(0),
        total_gold_units: gold.unwrap_or(0),
    };
    // An explicitly encoded empty text or zero is a second encoding of the default.
    if encode_npc_shop_result(&result)? != payload {
        return malformed();
    }
    Ok(result)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "npc_shop_tests.rs"]
mod tests;
