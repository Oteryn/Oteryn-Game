//! NPC service typed payloads (NPC-WIRE-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/npc_service_v1.proto`, from NPC-0 §4. Capability 3
//! `NPC_SERVICE_V1` gates command types 7 `NPC_TALK_INTENT` and 8 `NPC_TRADE_INTENT` and state
//! domains 7 `NPC_CONVERSATION` and 8 `NPC_TRADE_WINDOW`. The capability is registered and not
//! offered: the send path, the domain owner and the revision stream belong to NPC-TALK-1.
//!
//! Decoding is strict, and encoding refuses the same values before any byte is emitted: an NPC
//! whose generation is zero or whose identity is not 16 bytes, empty or control-character text,
//! a zero or unknown enum, packed scalars, unknown or repeated fields, and any count or byte size
//! over its bound all fail closed.

// The client-side codecs (domain decode, intent encode) are exercised by the tests; the server
// composes its own direction in `gameplay_transport`.
#![cfg_attr(not(test), allow(dead_code))]

pub use crate::charm_wire::CyclopediaWireError as NpcServiceWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes,
    read_result_enum, read_uint32, read_varint, set_once,
};
use crate::world_spatial_entities::{ENTITY_IDENTITY_BYTES, EntityRef};

/// Registered capability `NPC_SERVICE_V1`: command types 7 and 8, state domains 7 and 8.
pub const CAPABILITY_NPC_SERVICE_V1: u32 = 3;
pub const COMMAND_TYPE_NPC_TALK_INTENT: u32 = 7;
pub const COMMAND_TYPE_NPC_TRADE_INTENT: u32 = 8;
pub const STATE_DOMAIN_NPC_CONVERSATION: u32 = 7;
pub const STATE_DOMAIN_NPC_TRADE_WINDOW: u32 = 8;
pub const SNAPSHOT_TYPE_NPC_CONVERSATION_V1: u32 = 1;
pub const DELTA_TYPE_NPC_CONVERSATION_V1: u32 = 1;
pub const SNAPSHOT_TYPE_NPC_TRADE_WINDOW_V1: u32 = 1;
pub const DELTA_TYPE_NPC_TRADE_WINDOW_V1: u32 = 1;

/// `NPC0-RL-01`: UTF-8 bytes of one talk text.
pub const MAX_TALK_TEXT_BYTES: usize = 255;
/// `NPC0-RL-02`: reply lines of one domain 7 payload.
pub const MAX_REPLY_LINES: usize = 32;
/// `NPC0-RL-02-LINE-BYTES`: UTF-8 bytes of one reply line (measured 1,919 over the catalogue).
pub const MAX_REPLY_LINE_BYTES: usize = 2048;
/// `NPC0-RL-03`: offers of one catalogue projection (measured 757).
pub const MAX_OFFERS: usize = 1024;
/// `NPC0-RL-04`: talk intents of one GameSession per second.
pub const MAX_TALK_INTENTS_PER_SECOND: u32 = 4;
/// `NPC0-RL-05`: open conversations of one NPC.
pub const MAX_OPEN_CONVERSATIONS_PER_NPC: u32 = 8;
/// `NPC0-RL-06`: seconds a travel confirmation stays open.
pub const TRAVEL_CONFIRMATION_TIMEOUT_SECONDS: u32 = 30;
/// `NPC0-RL-07`: talk range in tiles (Chebyshev, same floor).
pub const TALK_RANGE_TILES: u32 = 4;

/// An `EntityRefV1` with a target: identity 1 + 1 + 16, generation 1 + 10.
const MAX_NPC_REF_BYTES: usize = 29;
/// A message field holding an NPC reference: its tag, a 1-byte length and the reference.
const MAX_NPC_FIELD_BYTES: usize = 2 + MAX_NPC_REF_BYTES;
/// `NpcTalkIntentV1`: the NPC 31 and the text 1 + 2 + 255.
pub const MAX_NPC_TALK_INTENT_BYTES: usize = MAX_NPC_FIELD_BYTES + 3 + MAX_TALK_TEXT_BYTES;
/// `NpcTradeIntentV1`: the NPC 31, the revision 1 + 10, the offer index 1 + 5, the side 1 + 1, the
/// quantity 1 + 5 and the expected price 1 + 5.
pub const MAX_NPC_TRADE_INTENT_BYTES: usize = MAX_NPC_FIELD_BYTES + 11 + 6 + 2 + 6 + 6;
/// `NpcIntentResultV1`: the disposition 1 + 1.
pub const MAX_NPC_INTENT_RESULT_BYTES: usize = 2;
/// `NPC0-RL-02-BYTES`: one `NpcConversationV1`: the NPC 31, the state 1 + 1 and 32 line elements
/// of a tag, a 2-byte length and the line.
pub const MAX_NPC_CONVERSATION_BYTES: usize =
    MAX_NPC_FIELD_BYTES + 2 + MAX_REPLY_LINES * (3 + MAX_REPLY_LINE_BYTES);
/// One `NpcOfferV1`: four fields of 1 + 5, 1 + 1, 1 + 5 and 1 + 5.
const MAX_OFFER_BYTES: usize = 20;
/// One offer element: its tag, a 1-byte length and the offer.
const MAX_OFFER_ELEMENT_BYTES: usize = 2 + MAX_OFFER_BYTES;
/// `NPC0-RL-03-BYTES`: one `NpcTradeWindowV1`: the NPC 31, the revision 1 + 10 and 1,024 offer
/// elements.
pub const MAX_NPC_TRADE_WINDOW_BYTES: usize =
    MAX_NPC_FIELD_BYTES + 11 + MAX_OFFERS * MAX_OFFER_ELEMENT_BYTES;

fn malformed<T>() -> WireResult<T> {
    Err(NpcServiceWireError::Malformed)
}

/// `NpcTradeSide`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcTradeSide {
    Buy = 1,
    Sell = 2,
}

impl NpcTradeSide {
    fn from_wire(value: u32) -> WireResult<Self> {
        match value {
            1 => Ok(Self::Buy),
            2 => Ok(Self::Sell),
            _ => malformed(),
        }
    }
}

/// `NpcIntentDisposition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcIntentDisposition {
    Ok = 1,
    NotInRange = 2,
    NoConversation = 3,
    RateLimited = 4,
    Stale = 5,
    Rejected = 6,
}

impl NpcIntentDisposition {
    pub const ALL: [Self; 6] = [
        Self::Ok,
        Self::NotInRange,
        Self::NoConversation,
        Self::RateLimited,
        Self::Stale,
        Self::Rejected,
    ];
}

/// `NpcConversationState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcConversationState {
    Open = 1,
    /// A travel confirmation is pending.
    Confirming = 2,
}

impl NpcConversationState {
    fn from_wire(value: u32) -> WireResult<Self> {
        match value {
            1 => Ok(Self::Open),
            2 => Ok(Self::Confirming),
            _ => malformed(),
        }
    }
}

/// `NpcTalkIntentV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcTalkIntent {
    pub npc_actor: EntityRef,
    pub text: String,
}

/// `NpcTradeIntentV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcTradeIntent {
    pub npc_actor: EntityRef,
    pub catalogue_revision: u64,
    /// Below [`MAX_OFFERS`].
    pub offer_index: u32,
    pub side: NpcTradeSide,
    /// Non-zero.
    pub quantity: u32,
    pub expected_unit_price: u32,
}

/// `NpcConversationV1`: domain 7. The default is no conversation and no line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NpcConversation {
    /// The open conversation's NPC and state; both or neither.
    pub open: Option<(EntityRef, NpcConversationState)>,
    /// Reply lines addressed to this character, at most [`MAX_REPLY_LINES`].
    pub lines: Vec<String>,
}

/// `NpcOfferV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcOffer {
    /// Non-zero.
    pub item_definition_ref: u32,
    pub side: NpcTradeSide,
    /// Non-zero.
    pub count: u32,
    pub unit_price: u32,
}

/// `NpcTradeWindowV1`: domain 8. The default is a closed window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NpcTradeWindow {
    /// The NPC and catalogue revision of an open window.
    pub open: Option<(EntityRef, u64)>,
    /// Offers in index order, at most [`MAX_OFFERS`]; empty for a closed window.
    pub offers: Vec<NpcOffer>,
}

fn check_npc(npc: &EntityRef) -> WireResult<()> {
    if npc.generation == 0 {
        return malformed();
    }
    Ok(())
}

fn check_size(payload: &[u8], maximum: usize) -> WireResult<()> {
    if payload.len() > maximum {
        return Err(NpcServiceWireError::LimitExceeded);
    }
    Ok(())
}

fn encode_npc(output: &mut Vec<u8>, field: u64, npc: &EntityRef) -> WireResult<()> {
    check_npc(npc)?;
    let mut inner = Vec::with_capacity(MAX_NPC_REF_BYTES);
    push_message_field(&mut inner, 1, &npc.identity);
    push_varint_field(&mut inner, 2, npc.generation);
    push_message_field(output, field, &inner);
    Ok(())
}

fn decode_npc(input: &[u8]) -> WireResult<EntityRef> {
    let (mut identity, mut generation) = (None, None);
    read_fields(input, |number, wire, input, cursor| match (number, wire) {
        (1, 2) => {
            let bytes: [u8; ENTITY_IDENTITY_BYTES] = read_bytes(input, cursor)?
                .try_into()
                .map_err(|_| NpcServiceWireError::Malformed)?;
            set_once(&mut identity, bytes)
        }
        (2, 0) => set_once(&mut generation, read_varint(input, cursor)?),
        _ => malformed(),
    })?;
    let npc = EntityRef {
        identity: identity.ok_or(NpcServiceWireError::Malformed)?,
        generation: generation.ok_or(NpcServiceWireError::Malformed)?,
    };
    check_npc(&npc)?;
    Ok(npc)
}

/// Reads the fields of a message: `field(number, wire_type, input, cursor)` consumes one value
/// or refuses the key.
fn read_fields(
    input: &[u8],
    mut field: impl FnMut(u64, u64, &[u8], &mut usize) -> WireResult<()>,
) -> WireResult<()> {
    let mut cursor = 0;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        field(key >> 3, key & 0x07, input, &mut cursor)?;
    }
    Ok(())
}

/// Non-empty text within `maximum` bytes without control characters.
fn check_text(text: &str, maximum: usize) -> WireResult<()> {
    if text.len() > maximum {
        return Err(NpcServiceWireError::LimitExceeded);
    }
    if text.is_empty() || text.chars().any(char::is_control) {
        return malformed();
    }
    Ok(())
}

fn read_text(input: &[u8], cursor: &mut usize, maximum: usize) -> WireResult<String> {
    let bytes = read_bytes(input, cursor)?;
    if bytes.len() > maximum {
        return Err(NpcServiceWireError::LimitExceeded);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| NpcServiceWireError::Malformed)?;
    check_text(text, maximum)?;
    Ok(text.to_owned())
}

fn check_offer_index(index: u32) -> WireResult<()> {
    if usize::try_from(index).map_or(true, |index| index >= MAX_OFFERS) {
        return Err(NpcServiceWireError::LimitExceeded);
    }
    Ok(())
}

/// Encodes an `NpcTalkIntentV1` (the client side).
pub fn encode_npc_talk_intent(intent: &NpcTalkIntent) -> WireResult<Vec<u8>> {
    check_text(&intent.text, MAX_TALK_TEXT_BYTES)?;
    let mut output = Vec::with_capacity(MAX_NPC_TALK_INTENT_BYTES);
    encode_npc(&mut output, 1, &intent.npc_actor)?;
    push_message_field(&mut output, 2, intent.text.as_bytes());
    Ok(output)
}

/// Decodes an `NpcTalkIntentV1`; the server answers `REJECTED` to any error.
pub fn decode_npc_talk_intent(payload: &[u8]) -> WireResult<NpcTalkIntent> {
    check_size(payload, MAX_NPC_TALK_INTENT_BYTES)?;
    let (mut npc, mut text) = (None, None);
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 2) => set_once(&mut npc, decode_npc(read_bytes(input, cursor)?)?),
            (2, 2) => set_once(&mut text, read_text(input, cursor, MAX_TALK_TEXT_BYTES)?),
            _ => malformed(),
        }
    })?;
    Ok(NpcTalkIntent {
        npc_actor: npc.ok_or(NpcServiceWireError::Malformed)?,
        text: text.ok_or(NpcServiceWireError::Malformed)?,
    })
}

fn check_trade_intent(intent: &NpcTradeIntent) -> WireResult<()> {
    check_npc(&intent.npc_actor)?;
    check_offer_index(intent.offer_index)?;
    if intent.quantity == 0 {
        return malformed();
    }
    Ok(())
}

/// Encodes an `NpcTradeIntentV1` (the client side).
pub fn encode_npc_trade_intent(intent: &NpcTradeIntent) -> WireResult<Vec<u8>> {
    check_trade_intent(intent)?;
    let mut output = Vec::with_capacity(MAX_NPC_TRADE_INTENT_BYTES);
    encode_npc(&mut output, 1, &intent.npc_actor)?;
    push_nonzero_varint_field(&mut output, 2, intent.catalogue_revision);
    push_nonzero_varint_field(&mut output, 3, u64::from(intent.offer_index));
    push_varint_field(&mut output, 4, intent.side as u64);
    push_varint_field(&mut output, 5, u64::from(intent.quantity));
    push_nonzero_varint_field(&mut output, 6, u64::from(intent.expected_unit_price));
    Ok(output)
}

/// Decodes an `NpcTradeIntentV1`; the server answers `REJECTED` to any error.
pub fn decode_npc_trade_intent(payload: &[u8]) -> WireResult<NpcTradeIntent> {
    check_size(payload, MAX_NPC_TRADE_INTENT_BYTES)?;
    let (mut npc, mut revision, mut index) = (None, None, None);
    let (mut side, mut quantity, mut price) = (None, None, None);
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 2) => set_once(&mut npc, decode_npc(read_bytes(input, cursor)?)?),
            (2, 0) => set_once(&mut revision, read_varint(input, cursor)?),
            (3, 0) => set_once(&mut index, read_uint32(input, cursor)?),
            (4, 0) => set_once(
                &mut side,
                NpcTradeSide::from_wire(read_uint32(input, cursor)?)?,
            ),
            (5, 0) => set_once(&mut quantity, read_uint32(input, cursor)?),
            (6, 0) => set_once(&mut price, read_uint32(input, cursor)?),
            _ => malformed(),
        }
    })?;
    let intent = NpcTradeIntent {
        npc_actor: npc.ok_or(NpcServiceWireError::Malformed)?,
        catalogue_revision: revision.unwrap_or(0),
        offer_index: index.unwrap_or(0),
        side: side.ok_or(NpcServiceWireError::Malformed)?,
        quantity: quantity.unwrap_or(0),
        expected_unit_price: price.unwrap_or(0),
    };
    check_trade_intent(&intent)?;
    Ok(intent)
}

/// Encodes an `NpcIntentResultV1` (the server side).
pub fn encode_npc_intent_result(disposition: NpcIntentDisposition) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_NPC_INTENT_RESULT_BYTES);
    push_varint_field(&mut output, 1, disposition as u64);
    output
}

pub fn decode_npc_intent_result(payload: &[u8]) -> WireResult<NpcIntentDisposition> {
    let value = read_result_enum(payload, MAX_NPC_INTENT_RESULT_BYTES)?;
    NpcIntentDisposition::ALL
        .into_iter()
        .find(|disposition| *disposition as u32 == value)
        .ok_or(NpcServiceWireError::Malformed)
}

fn check_conversation(conversation: &NpcConversation) -> WireResult<()> {
    if let Some((npc, _)) = &conversation.open {
        check_npc(npc)?;
    }
    if conversation.lines.len() > MAX_REPLY_LINES {
        return Err(NpcServiceWireError::LimitExceeded);
    }
    conversation
        .lines
        .iter()
        .try_for_each(|line| check_text(line, MAX_REPLY_LINE_BYTES))
}

/// Encodes the snapshot (type 1) or delta (type 1) payload of domain 7.
pub fn encode_npc_conversation(conversation: &NpcConversation) -> WireResult<Vec<u8>> {
    check_conversation(conversation)?;
    let mut output = Vec::with_capacity(MAX_NPC_CONVERSATION_BYTES);
    if let Some((npc, state)) = &conversation.open {
        encode_npc(&mut output, 1, npc)?;
        push_varint_field(&mut output, 2, *state as u64);
    }
    for line in &conversation.lines {
        push_message_field(&mut output, 3, line.as_bytes());
    }
    Ok(output)
}

pub fn decode_npc_conversation(payload: &[u8]) -> WireResult<NpcConversation> {
    check_size(payload, MAX_NPC_CONVERSATION_BYTES)?;
    let (mut npc, mut state, mut lines) = (None, None, Vec::new());
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 2) => set_once(&mut npc, decode_npc(read_bytes(input, cursor)?)?),
            (2, 0) => set_once(
                &mut state,
                NpcConversationState::from_wire(read_uint32(input, cursor)?)?,
            ),
            (3, 2) => {
                if lines.len() == MAX_REPLY_LINES {
                    return Err(NpcServiceWireError::LimitExceeded);
                }
                lines.push(read_text(input, cursor, MAX_REPLY_LINE_BYTES)?);
                Ok(())
            }
            _ => malformed(),
        }
    })?;
    let open = match (npc, state) {
        (Some(npc), Some(state)) => Some((npc, state)),
        (None, None) => None,
        _ => return malformed(),
    };
    Ok(NpcConversation { open, lines })
}

fn check_offer(offer: &NpcOffer) -> WireResult<()> {
    if offer.item_definition_ref == 0 || offer.count == 0 {
        return malformed();
    }
    Ok(())
}

fn encode_offer(offer: &NpcOffer) -> WireResult<Vec<u8>> {
    check_offer(offer)?;
    let mut output = Vec::with_capacity(MAX_OFFER_BYTES);
    push_varint_field(&mut output, 1, u64::from(offer.item_definition_ref));
    push_varint_field(&mut output, 2, offer.side as u64);
    push_varint_field(&mut output, 3, u64::from(offer.count));
    push_nonzero_varint_field(&mut output, 4, u64::from(offer.unit_price));
    Ok(output)
}

fn decode_offer(input: &[u8]) -> WireResult<NpcOffer> {
    check_size(input, MAX_OFFER_BYTES)?;
    let (mut item, mut side, mut count, mut price) = (None, None, None, None);
    read_fields(input, |number, wire, input, cursor| match (number, wire) {
        (1, 0) => set_once(&mut item, read_uint32(input, cursor)?),
        (2, 0) => set_once(
            &mut side,
            NpcTradeSide::from_wire(read_uint32(input, cursor)?)?,
        ),
        (3, 0) => set_once(&mut count, read_uint32(input, cursor)?),
        (4, 0) => set_once(&mut price, read_uint32(input, cursor)?),
        _ => malformed(),
    })?;
    let offer = NpcOffer {
        item_definition_ref: item.unwrap_or(0),
        side: side.ok_or(NpcServiceWireError::Malformed)?,
        count: count.unwrap_or(0),
        unit_price: price.unwrap_or(0),
    };
    check_offer(&offer)?;
    Ok(offer)
}

fn check_window(window: &NpcTradeWindow) -> WireResult<()> {
    if let Some((npc, _)) = &window.open {
        check_npc(npc)?;
    } else if !window.offers.is_empty() {
        return malformed();
    }
    if window.offers.len() > MAX_OFFERS {
        return Err(NpcServiceWireError::LimitExceeded);
    }
    window.offers.iter().try_for_each(check_offer)
}

/// Encodes the snapshot (type 1) or delta (type 1) payload of domain 8.
pub fn encode_npc_trade_window(window: &NpcTradeWindow) -> WireResult<Vec<u8>> {
    check_window(window)?;
    let mut output = Vec::with_capacity(MAX_NPC_TRADE_WINDOW_BYTES);
    if let Some((npc, revision)) = &window.open {
        encode_npc(&mut output, 1, npc)?;
        push_nonzero_varint_field(&mut output, 2, *revision);
    }
    for offer in &window.offers {
        push_message_field(&mut output, 3, &encode_offer(offer)?);
    }
    Ok(output)
}

pub fn decode_npc_trade_window(payload: &[u8]) -> WireResult<NpcTradeWindow> {
    check_size(payload, MAX_NPC_TRADE_WINDOW_BYTES)?;
    let (mut npc, mut revision, mut offers) = (None, None, Vec::new());
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 2) => set_once(&mut npc, decode_npc(read_bytes(input, cursor)?)?),
            (2, 0) => set_once(&mut revision, read_varint(input, cursor)?),
            (3, 2) => {
                if offers.len() == MAX_OFFERS {
                    return Err(NpcServiceWireError::LimitExceeded);
                }
                offers.push(decode_offer(read_bytes(input, cursor)?)?);
                Ok(())
            }
            _ => malformed(),
        }
    })?;
    let window = NpcTradeWindow {
        open: match (npc, revision) {
            (Some(npc), revision) => Some((npc, revision.unwrap_or(0))),
            (None, None) => None,
            (None, Some(_)) => return malformed(),
        },
        offers,
    };
    check_window(&window)?;
    Ok(window)
}

#[cfg(test)]
#[path = "npc_service_tests.rs"]
mod tests;
