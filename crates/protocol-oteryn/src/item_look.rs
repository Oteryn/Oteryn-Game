//! CP D1240 ITEM-LOOK-1 codec with capability28 selection gating; no runtime offer.
//!
//! The Item branch retains ItemTargetV1 bytes; Creature carries the existing native
//! spatial EntityRef. Results are UTF-8 text or one of four fact-free refusals.
//! CP D1240 accepts these payload fields. Runtime visibility/name binding remains separate.

use crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES;
use crate::charm_wire::push_varint;
use crate::charm_wire::{push_message_field, read_bytes, read_varint, set_once};
pub use crate::item_view::ItemViewWireError as ItemLookWireError;
use crate::item_view::{ItemHandle, decode_item_target, encode_item_target};
use crate::world_spatial_entities::EntityRef;
use std::num::NonZeroU64;

/// CP D1240 assigns capability28; no offer or dispatcher is installed here.
pub const ITEM_LOOK_CAPABILITY_ID: Option<u32> = Some(28);
/// CP D1268 reserves command30; activation waits for the final shared MQ slot.
pub const ITEM_LOOK_COMMAND_ID: Option<u32> = Some(30);

/// D1143's result text limit in UTF-8 bytes, excluding the existing wire envelope framing.
pub const MAX_ITEM_LOOK_TEXT_BYTES: usize = 512;

/// Actual session negotiation must supply this selection; default retains legacy bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemLookSelection {
    /// Capability28: Creature request branch and extended result refusals.
    pub extended: bool,
}

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

pub fn encode_item_look_intent(intent: ItemLookIntent) -> Result<Vec<u8>, ItemLookWireError> {
    encode_item_look_intent_for(intent, ItemLookSelection::default())
}

pub fn encode_item_look_intent_for(
    intent: ItemLookIntent,
    selection: ItemLookSelection,
) -> Result<Vec<u8>, ItemLookWireError> {
    match intent.target {
        ItemLookTarget::Item(handle) => Ok(encode_item_target(handle)),
        ItemLookTarget::Creature(_) if !selection.extended => Err(ItemLookWireError::Malformed),
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
            Ok(output)
        }
    }
}

pub fn decode_item_look_intent(payload: &[u8]) -> Result<ItemLookIntent, ItemLookWireError> {
    decode_item_look_intent_for(payload, ItemLookSelection::default())
}

pub fn decode_item_look_intent_for(
    payload: &[u8],
    selection: ItemLookSelection,
) -> Result<ItemLookIntent, ItemLookWireError> {
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
    if tag != 0x12 || !selection.extended {
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

/// The server maps extended refusals to legacy NotAvailable without capability28.
pub fn encode_item_look_result(result: &ItemLookResult) -> Result<Vec<u8>, ItemLookWireError> {
    encode_item_look_result_for(result, ItemLookSelection::default())
}

pub fn encode_item_look_result_for(
    result: &ItemLookResult,
    selection: ItemLookSelection,
) -> Result<Vec<u8>, ItemLookWireError> {
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
                _ if !selection.extended => 2,
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
    decode_item_look_result_for(payload, ItemLookSelection::default())
}

pub fn decode_item_look_result_for(
    payload: &[u8],
    selection: ItemLookSelection,
) -> Result<ItemLookResult, ItemLookWireError> {
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
                if tag != 0x12 && !selection.extended {
                    return Err(ItemLookWireError::Malformed);
                }
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

/// D1240's mandatory unknown-generation refusal before runtime census lookup.
/// A nonzero result is only a known generation value: it grants no current-actor,
/// visibility, reach, profile or name binding. The runtime producer must prove those.
pub fn known_creature_generation(reference: EntityRef) -> Result<NonZeroU64, ItemLookRefusal> {
    NonZeroU64::new(reference.generation).ok_or(ItemLookRefusal::NotVisible)
}

#[cfg(test)]
#[path = "item_look_tests.rs"]
mod tests;
