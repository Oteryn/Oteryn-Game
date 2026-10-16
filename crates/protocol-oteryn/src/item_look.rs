//! Registered ITEM-LOOK-1 codec proposal; architect acceptance and CP-leased IDs are pending.
//!
//! The Item branch retains ItemTargetV1 bytes; Creature carries the existing native
//! spatial EntityRef. Results are UTF-8 text or one of four fact-free refusals.
//! These local field choices remain subject to CP protocol review. No runtime offer is made.

use crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES;
use crate::charm_wire::push_varint;
use crate::charm_wire::{push_message_field, read_bytes, read_varint, set_once};
pub use crate::item_view::ItemViewWireError as ItemLookWireError;
use crate::item_view::{ItemHandle, decode_item_target, encode_item_target};
use crate::world_spatial_entities::EntityRef;

/// Intentionally unassigned: these are not negotiated or dispatchable identifiers.
pub const ITEM_LOOK_CAPABILITY_ID: Option<u32> = None;
pub const ITEM_LOOK_COMMAND_ID: Option<u32> = None;

/// D1143's result text limit in UTF-8 bytes, excluding the existing wire envelope framing.
pub const MAX_ITEM_LOOK_TEXT_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemLookIntent {
    pub target: ItemLookTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemLookTarget {
    Item(ItemHandle),
    /// Native visible-entity identity and actor generation; not a new Creature ID.
    Creature(EntityRef),
}

/// Largest Creature target: outer framing + 16 identity bytes + full u64 generation.
pub const MAX_ITEM_LOOK_TARGET_BYTES: usize = 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemLookRefusal {
    NotVisible,
    NotReachable,
    RateLimited,
    NotAvailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemLookResult {
    Text(String),
    /// No identity, visibility, name or other target facts are carried by a refusal.
    Refused(ItemLookRefusal),
}

pub fn encode_item_look_intent(intent: ItemLookIntent) -> Vec<u8> {
    match intent.target {
        ItemLookTarget::Item(handle) => encode_item_target(handle),
        ItemLookTarget::Creature(entity) => {
            // Same EntityRef field layout and zero-generation default as spatial snapshots.
            let mut reference = Vec::new();
            push_message_field(&mut reference, 1, &entity.identity);
            if entity.generation != 0 {
                push_varint(&mut reference, 0x10);
                push_varint(&mut reference, entity.generation);
            }
            let mut output = Vec::new();
            push_message_field(&mut output, 2, &reference);
            output
        }
    }
}

pub fn decode_item_look_intent(payload: &[u8]) -> Result<ItemLookIntent, ItemLookWireError> {
    if payload.len() > MAX_ITEM_LOOK_TARGET_BYTES {
        return Err(ItemLookWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let tag = read_varint(payload, &mut cursor)?;
    // Preserve the existing ItemTargetV1 branch, including its own exact size bound,
    // zero-handle refusal and strict duplicate/trailing-field handling.
    if tag == 0x08 {
        return Ok(ItemLookIntent {
            target: ItemLookTarget::Item(decode_item_target(payload)?),
        });
    }
    if tag != 0x12 {
        return Err(ItemLookWireError::Malformed);
    }
    let reference = read_bytes(payload, &mut cursor)?;
    if cursor != payload.len() {
        return Err(ItemLookWireError::Malformed);
    }
    let mut cursor = 0;
    let (mut identity, mut generation) = (None, None);
    while cursor < reference.len() {
        match read_varint(reference, &mut cursor)? {
            0x0a => {
                let bytes = read_bytes(reference, &mut cursor)?;
                let bytes: [u8; 16] = bytes.try_into().map_err(|_| ItemLookWireError::Malformed)?;
                set_once(&mut identity, bytes)?;
            }
            0x10 => set_once(&mut generation, read_varint(reference, &mut cursor)?)?,
            _ => return Err(ItemLookWireError::Malformed),
        }
    }
    Ok(ItemLookIntent {
        target: ItemLookTarget::Creature(EntityRef {
            identity: identity.ok_or(ItemLookWireError::Malformed)?,
            generation: generation.unwrap_or(0),
        }),
    })
}

pub fn encode_item_look_result(result: &ItemLookResult) -> Result<Vec<u8>, ItemLookWireError> {
    if let ItemLookResult::Text(text) = result {
        if text.is_empty() {
            return Err(ItemLookWireError::Malformed);
        }
        if text.len() > MAX_ITEM_LOOK_TEXT_BYTES || text.len() > MAX_COMMAND_RESULT_PAYLOAD_BYTES {
            return Err(ItemLookWireError::LimitExceeded);
        }
    }
    // Refuse oversized text before allocating output. The final check retains envelope framing.
    let mut output = Vec::new();
    match result {
        ItemLookResult::Text(text) => {
            push_message_field(&mut output, 1, text.as_bytes());
        }
        ItemLookResult::Refused(refusal) => {
            let field = match refusal {
                ItemLookRefusal::NotAvailable => 2,
                ItemLookRefusal::NotVisible => 3,
                ItemLookRefusal::NotReachable => 4,
                ItemLookRefusal::RateLimited => 5,
            };
            push_message_field(&mut output, field, &[]);
        }
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
                if text.len() > MAX_ITEM_LOOK_TEXT_BYTES {
                    return Err(ItemLookWireError::LimitExceeded);
                }
                ItemLookResult::Text(text.to_owned())
            }
            tag @ (0x12 | 0x1a | 0x22 | 0x2a) => {
                if !read_bytes(payload, &mut cursor)?.is_empty() {
                    return Err(ItemLookWireError::Malformed);
                }
                ItemLookResult::Refused(match tag {
                    0x12 => ItemLookRefusal::NotAvailable,
                    0x1a => ItemLookRefusal::NotVisible,
                    0x22 => ItemLookRefusal::NotReachable,
                    _ => ItemLookRefusal::RateLimited,
                })
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
