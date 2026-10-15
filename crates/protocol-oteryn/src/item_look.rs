//! Unregistered ITEM-LOOK-1 proposal; architect acceptance and CP-leased IDs are pending.
//!
//! The request reuses ItemTargetV1. The proposed result is a strict oneof: field 1 is
//! server-authored UTF-8 text, field 2 an empty unavailable marker. No runtime offer is made.

use crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES;
use crate::charm_wire::{push_message_field, read_bytes, read_varint, set_once};
pub use crate::item_view::ItemViewWireError as ItemLookWireError;
use crate::item_view::{ItemHandle, decode_item_target, encode_item_target};

/// Intentionally unassigned: these are not negotiated or dispatchable identifiers.
pub const ITEM_LOOK_CAPABILITY_ID: Option<u32> = None;
pub const ITEM_LOOK_COMMAND_ID: Option<u32> = None;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemLookIntent {
    pub target: ItemHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemLookResult {
    Text(String),
    /// Shared by unknown, hidden, stale and unadmitted targets; contains no target facts.
    Unavailable,
}

pub fn encode_item_look_intent(intent: ItemLookIntent) -> Vec<u8> {
    encode_item_target(intent.target)
}

pub fn decode_item_look_intent(payload: &[u8]) -> Result<ItemLookIntent, ItemLookWireError> {
    Ok(ItemLookIntent {
        target: decode_item_target(payload)?,
    })
}

pub fn encode_item_look_result(result: &ItemLookResult) -> Result<Vec<u8>, ItemLookWireError> {
    let mut output = Vec::new();
    match result {
        ItemLookResult::Text(text) => {
            if text.is_empty() {
                return Err(ItemLookWireError::Malformed);
            }
            // Refuse before allocating the output. The final check includes tag/length bytes.
            if text.len() > MAX_COMMAND_RESULT_PAYLOAD_BYTES {
                return Err(ItemLookWireError::LimitExceeded);
            }
            push_message_field(&mut output, 1, text.as_bytes());
        }
        ItemLookResult::Unavailable => push_message_field(&mut output, 2, &[]),
    }
    if output.len() > MAX_COMMAND_RESULT_PAYLOAD_BYTES {
        return Err(ItemLookWireError::LimitExceeded);
    }
    Ok(output)
}

pub fn decode_item_look_result(payload: &[u8]) -> Result<ItemLookResult, ItemLookWireError> {
    if payload.len() > MAX_COMMAND_RESULT_PAYLOAD_BYTES {
        return Err(ItemLookWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut result = None;
    while cursor < payload.len() {
        let value = match read_varint(payload, &mut cursor)? {
            0x0a => {
                let bytes = read_bytes(payload, &mut cursor)?;
                let text = std::str::from_utf8(bytes).map_err(|_| ItemLookWireError::Malformed)?;
                if text.is_empty() {
                    return Err(ItemLookWireError::Malformed);
                }
                ItemLookResult::Text(text.to_owned())
            }
            0x12 => {
                if !read_bytes(payload, &mut cursor)?.is_empty() {
                    return Err(ItemLookWireError::Malformed);
                }
                ItemLookResult::Unavailable
            }
            _ => return Err(ItemLookWireError::Malformed),
        };
        set_once(&mut result, value)?;
    }
    result.ok_or(ItemLookWireError::Malformed)
}

#[cfg(test)]
#[path = "item_look_tests.rs"]
mod tests;
