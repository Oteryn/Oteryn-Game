//! Item view and item move codecs (ITEM-VIEW-1a).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/item_view_v1.proto`, from ITEM-MOVE-WIRE-0 §4 and §5
//! (owner D212). Capability 4 `ITEM_VIEW_MOVE_V1` (requires capability 6) gates state domains 9
//! `CHARACTER_INVENTORY` and 11 `OPEN_CONTAINER`, command type 9 `ITEM_MOVE_INTENT`, the USE item
//! target (`world_object`) and the item handle of the D85 entry (`world_spatial_entities`). The
//! server does not offer the capability before ITEM-MOVE-1; this module has no server behaviour.
//!
//! Decoding is strict, and encoding refuses the same values before any byte is emitted: a zero
//! handle, item definition or count, a handle repeated within one view, entries without their
//! backpack or container, an empty or unknown destination, zero or unknown enums, unknown or
//! repeated fields and any count or byte size over its bound all fail closed.

// The client-side codecs (view decode, intent encode) are exercised by the tests; the server
// composes its own direction in ITEM-VIEW-1b and ITEM-MOVE-1.
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::BTreeSet;
use std::num::{NonZeroU32, NonZeroU64};

pub use crate::charm_wire::CyclopediaWireError as ItemViewWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes,
    read_result_enum, read_uint32, read_varint, set_once,
};

/// Registered capability `ITEM_VIEW_MOVE_V1`: domains 9 and 11, command type 9, the USE item
/// target and the D85 item handle.
pub const CAPABILITY_ITEM_VIEW_MOVE_V1: u32 = 4;
/// Capability 4 requires `WORLD_SPATIAL_ENTITIES`, which carries corpses in domain 1.
pub const CAPABILITY_ITEM_VIEW_MOVE_V1_REQUIRES: &[u32] =
    &[crate::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES];
pub const STATE_DOMAIN_CHARACTER_INVENTORY: u32 = 9;
pub const SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1: u32 = 1;
pub const DELTA_TYPE_CHARACTER_INVENTORY_V1: u32 = 1;
pub const STATE_DOMAIN_OPEN_CONTAINER: u32 = 11;
pub const SNAPSHOT_TYPE_OPEN_CONTAINER_V1: u32 = 1;
pub const DELTA_TYPE_OPEN_CONTAINER_V1: u32 = 1;
pub const COMMAND_TYPE_ITEM_MOVE_INTENT: u32 = 9;

/// `ITEMV0-RL-01`: items in one domain-9 view, the main backpack included (ITEM-MOVE-WIRE-1 §6.3
/// supersedes WIRE-0's 21 so the nine equipment slots fit later).
pub const MAX_CHARACTER_INVENTORY_ITEMS: usize = 30;
/// `ITEMV0-RL-02`: entries in one domain-11 view, the corpse container capacity
/// (`GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX`).
pub const MAX_OPEN_CONTAINER_ENTRIES: usize = 16;
/// `ITEMV0-RL-03`: live handles of one session. A handle is live while its item is in one of the
/// session's views: domain 1 shows at most 255 objects besides the own player (`MOVE-RL-11`),
/// domain 9 at most [`MAX_CHARACTER_INVENTORY_ITEMS`] and domain 11 at most
/// [`MAX_OPEN_CONTAINER_ENTRIES`] entries; the open corpse keeps its domain-1 handle.
pub const MAX_LIVE_ITEM_HANDLES: usize = 301;

/// One `ItemEntryV1`, measured: handle 1 + 10, definition 1 + 5, count 1 + 5, sub-type 1 + 5.
pub const MAX_ITEM_ENTRY_BYTES: usize = 29;
/// One `ItemEntryV1` as an element of a repeated field: its tag, a 1-byte length and the entry.
const MAX_ITEM_ENTRY_ELEMENT_BYTES: usize = 2 + MAX_ITEM_ENTRY_BYTES;
/// Domain 9 snapshot and delta: 30 entry elements.
pub const MAX_CHARACTER_INVENTORY_BYTES: usize =
    MAX_CHARACTER_INVENTORY_ITEMS * MAX_ITEM_ENTRY_ELEMENT_BYTES;
/// Domain 11 snapshot and delta: the container handle 1 + 10 and 16 entry elements.
pub const MAX_OPEN_CONTAINER_BYTES: usize =
    11 + MAX_OPEN_CONTAINER_ENTRIES * MAX_ITEM_ENTRY_ELEMENT_BYTES;
/// `ItemTargetV1`: the handle 1 + 10.
pub const MAX_ITEM_TARGET_BYTES: usize = 11;
/// `ItemMoveIntentV1`: the source handle 1 + 10 and the empty `main_backpack` destination 1 + 1.
pub const MAX_ITEM_MOVE_INTENT_BYTES: usize = 13;
/// `ItemMoveResultV1`: one small enum, with the same slack as the other results.
pub const MAX_ITEM_MOVE_RESULT_BYTES: usize = 4;

/// A server-assigned item handle (§4.1): non-zero, monotonic per GameSession, never reused.
pub type ItemHandle = NonZeroU64;

/// `ItemEntryV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemEntry {
    pub handle: ItemHandle,
    pub item_definition_ref: NonZeroU32,
    /// The whole stack.
    pub count: NonZeroU32,
    /// 0 when the item has no sub-type.
    pub sub_type: u32,
}

/// `CharacterInventoryV1`: the main backpack slot and its direct entries in display order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterInventory {
    pub main_backpack: Option<ItemEntry>,
    pub entries: Vec<ItemEntry>,
}

/// `OpenContainerV1`: the one open corpse and its entries; the default means nothing is open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenContainer {
    pub container_handle: Option<ItemHandle>,
    pub entries: Vec<ItemEntry>,
}

/// The `ItemMoveIntentV1.destination` oneof; this slice registers only the main backpack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemMoveDestination {
    MainBackpack,
}

/// `ItemMoveIntentV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemMoveIntent {
    pub source: ItemHandle,
    pub destination: ItemMoveDestination,
}

/// `ItemMoveOutcomeV1`, the WIRE-0 §5 results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemMoveOutcome {
    Moved = 1,
    Stale = 2,
    TooFar = 3,
    NoBackpack = 4,
    NoRoom = 5,
    NotOwner = 6,
    NotPickupable = 7,
    NotSupported = 8,
    Rejected = 9,
}

fn malformed<T>() -> WireResult<T> {
    Err(ItemViewWireError::Malformed)
}

fn nonzero_u64(value: u64) -> WireResult<ItemHandle> {
    NonZeroU64::new(value).ok_or(ItemViewWireError::Malformed)
}

fn nonzero_u32(value: u32) -> WireResult<NonZeroU32> {
    NonZeroU32::new(value).ok_or(ItemViewWireError::Malformed)
}

/// `ItemTargetV1`, the USE field 2 body.
pub fn encode_item_target(handle: ItemHandle) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_ITEM_TARGET_BYTES);
    push_varint_field(&mut output, 1, handle.get());
    output
}

/// A missing (zero) handle fails closed: there is no item without one.
pub fn decode_item_target(input: &[u8]) -> WireResult<ItemHandle> {
    if input.len() > MAX_ITEM_TARGET_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut handle = None;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 => set_once(&mut handle, read_varint(input, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    nonzero_u64(handle.unwrap_or(0))
}

fn encode_entry(output: &mut Vec<u8>, field: u64, entry: &ItemEntry) {
    let mut body = Vec::with_capacity(MAX_ITEM_ENTRY_BYTES);
    push_varint_field(&mut body, 1, entry.handle.get());
    push_varint_field(&mut body, 2, u64::from(entry.item_definition_ref.get()));
    push_varint_field(&mut body, 3, u64::from(entry.count.get()));
    push_nonzero_varint_field(&mut body, 4, u64::from(entry.sub_type));
    push_message_field(output, field, &body);
}

fn decode_entry(input: &[u8]) -> WireResult<ItemEntry> {
    if input.len() > MAX_ITEM_ENTRY_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut handle, mut definition, mut count, mut sub_type) = (None, None, None, None);
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 => set_once(&mut handle, read_varint(input, &mut cursor)?)?,
            0x10 => set_once(&mut definition, read_uint32(input, &mut cursor)?)?,
            0x18 => set_once(&mut count, read_uint32(input, &mut cursor)?)?,
            0x20 => set_once(&mut sub_type, read_uint32(input, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    Ok(ItemEntry {
        handle: nonzero_u64(handle.unwrap_or(0))?,
        item_definition_ref: nonzero_u32(definition.unwrap_or(0))?,
        count: nonzero_u32(count.unwrap_or(0))?,
        sub_type: sub_type.unwrap_or(0),
    })
}

/// Every handle appears once in a view, the holder's included.
fn unique_handles<'a>(
    holder: Option<ItemHandle>,
    entries: impl Iterator<Item = &'a ItemEntry>,
) -> WireResult<()> {
    let mut seen = BTreeSet::new();
    for handle in holder.into_iter().chain(entries.map(|entry| entry.handle)) {
        if !seen.insert(handle) {
            return malformed();
        }
    }
    Ok(())
}

fn validate_character_inventory(view: &CharacterInventory) -> WireResult<()> {
    if usize::from(view.main_backpack.is_some()) + view.entries.len()
        > MAX_CHARACTER_INVENTORY_ITEMS
    {
        return Err(ItemViewWireError::LimitExceeded);
    }
    if view.main_backpack.is_none() && !view.entries.is_empty() {
        return malformed();
    }
    unique_handles(
        view.main_backpack.map(|backpack| backpack.handle),
        view.entries.iter(),
    )
}

fn validate_open_container(view: &OpenContainer) -> WireResult<()> {
    if view.entries.len() > MAX_OPEN_CONTAINER_ENTRIES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    if view.container_handle.is_none() && !view.entries.is_empty() {
        return malformed();
    }
    unique_handles(view.container_handle, view.entries.iter())
}

/// Domain 9, snapshot type 1 and delta type 1 (the whole view).
pub fn encode_character_inventory(view: &CharacterInventory) -> WireResult<Vec<u8>> {
    validate_character_inventory(view)?;
    let mut output = Vec::new();
    if let Some(backpack) = &view.main_backpack {
        encode_entry(&mut output, 1, backpack);
    }
    for entry in &view.entries {
        encode_entry(&mut output, 2, entry);
    }
    Ok(output)
}

pub fn decode_character_inventory(payload: &[u8]) -> WireResult<CharacterInventory> {
    if payload.len() > MAX_CHARACTER_INVENTORY_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut view = CharacterInventory::default();
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x0a => set_once(
                &mut view.main_backpack,
                decode_entry(read_bytes(payload, &mut cursor)?)?,
            )?,
            0x12 => {
                // Entries need the backpack, which takes one of the 30.
                if view.entries.len() == MAX_CHARACTER_INVENTORY_ITEMS - 1 {
                    return Err(ItemViewWireError::LimitExceeded);
                }
                view.entries
                    .push(decode_entry(read_bytes(payload, &mut cursor)?)?);
            }
            _ => return malformed(),
        }
    }
    validate_character_inventory(&view)?;
    Ok(view)
}

/// Domain 11, snapshot type 1 and delta type 1 (the whole view).
pub fn encode_open_container(view: &OpenContainer) -> WireResult<Vec<u8>> {
    validate_open_container(view)?;
    let mut output = Vec::new();
    if let Some(handle) = view.container_handle {
        push_varint_field(&mut output, 1, handle.get());
    }
    for entry in &view.entries {
        encode_entry(&mut output, 2, entry);
    }
    Ok(output)
}

pub fn decode_open_container(payload: &[u8]) -> WireResult<OpenContainer> {
    if payload.len() > MAX_OPEN_CONTAINER_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut handle = None;
    let mut entries = Vec::new();
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x08 => set_once(&mut handle, read_varint(payload, &mut cursor)?)?,
            0x12 => {
                if entries.len() == MAX_OPEN_CONTAINER_ENTRIES {
                    return Err(ItemViewWireError::LimitExceeded);
                }
                entries.push(decode_entry(read_bytes(payload, &mut cursor)?)?);
            }
            _ => return malformed(),
        }
    }
    let view = OpenContainer {
        container_handle: handle.map(nonzero_u64).transpose()?,
        entries,
    };
    validate_open_container(&view)?;
    Ok(view)
}

/// `ClientCommand.payload` of command type 9.
pub fn encode_item_move_intent(intent: &ItemMoveIntent) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_ITEM_MOVE_INTENT_BYTES);
    push_varint_field(&mut output, 1, intent.source.get());
    match intent.destination {
        ItemMoveDestination::MainBackpack => push_message_field(&mut output, 2, &[]),
    }
    output
}

/// A missing source handle or destination, a non-empty `main_backpack` body and any other field
/// (the destinations a later capability adds) fail closed.
pub fn decode_item_move_intent(payload: &[u8]) -> WireResult<ItemMoveIntent> {
    if payload.len() > MAX_ITEM_MOVE_INTENT_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut source, mut destination) = (None, None);
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x08 => set_once(&mut source, read_varint(payload, &mut cursor)?)?,
            0x12 => {
                if !read_bytes(payload, &mut cursor)?.is_empty() {
                    return malformed();
                }
                set_once(&mut destination, ItemMoveDestination::MainBackpack)?;
            }
            _ => return malformed(),
        }
    }
    Ok(ItemMoveIntent {
        source: nonzero_u64(source.unwrap_or(0))?,
        destination: destination.ok_or(ItemViewWireError::Malformed)?,
    })
}

/// `CommandResult.payload` of command type 9.
pub fn encode_item_move_result(outcome: ItemMoveOutcome) -> Vec<u8> {
    let mut output = Vec::with_capacity(2);
    push_varint_field(&mut output, 1, outcome as u64);
    output
}

pub fn decode_item_move_result(payload: &[u8]) -> WireResult<ItemMoveOutcome> {
    Ok(
        match read_result_enum(payload, MAX_ITEM_MOVE_RESULT_BYTES)? {
            1 => ItemMoveOutcome::Moved,
            2 => ItemMoveOutcome::Stale,
            3 => ItemMoveOutcome::TooFar,
            4 => ItemMoveOutcome::NoBackpack,
            5 => ItemMoveOutcome::NoRoom,
            6 => ItemMoveOutcome::NotOwner,
            7 => ItemMoveOutcome::NotPickupable,
            8 => ItemMoveOutcome::NotSupported,
            9 => ItemMoveOutcome::Rejected,
            _ => return malformed(),
        },
    )
}

#[cfg(test)]
#[path = "item_view_tests.rs"]
mod tests;
