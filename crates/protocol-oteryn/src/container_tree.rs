//! Container view codecs (BAGS-WIRE-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/container_tree_v1.proto`, from BAGS-0 §5. Capability
//! 14 `CONTAINER_TREE_V1` requires capabilities 4 and 12 and gates state domain 14
//! `CONTAINER_VIEWS`, command type 21 `CONTAINER_VIEW_INTENT`, the `CONTAINER` destination of
//! command type 9 (`item_view`, [`crate::item_view::ItemMoveSelection`]) and USE field 2 on a
//! container handle. The server does not offer the capability before BAGS-1.
//!
//! Decoding is strict, and encoding refuses the same values before any byte is emitted: a zero
//! handle, item definition, count or capacity, a view id above 15, views out of ascending order,
//! more entries than the view's capacity, a handle repeated where it must be unique, an empty or
//! unknown action, zero or unknown enums, unknown or repeated fields and any count or byte size
//! over its bound all fail closed.

// The client-side codecs (view decode, intent encode) are exercised by the tests; the server
// composes its own direction in `gameplay_transport::container_view`.
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::BTreeSet;

use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes,
    read_result_enum, read_uint32, read_uint32_fields, read_varint, set_once,
};
use crate::item_view::{
    CAPABILITY_ITEM_EQUIP_DROP_V1, CAPABILITY_ITEM_VIEW_MOVE_V1, ItemEntry, ItemHandle,
    ItemViewWireError, MAX_ITEM_ENTRY_BYTES, MAX_ITEM_TARGET_BYTES, MAX_LIVE_ITEM_HANDLES,
    decode_entry, encode_entry,
};

/// Registered capability `CONTAINER_TREE_V1`: domain 14, command type 21, the `CONTAINER`
/// destination of command type 9 and USE field 2 on a container handle.
pub const CAPABILITY_CONTAINER_TREE_V1: u32 = 14;
/// BAGS-0 §5: capability 14 requires capability 4 and `ITEM_EQUIP_DROP_V1`.
pub const CAPABILITY_CONTAINER_TREE_V1_REQUIRES: &[u32] =
    &[CAPABILITY_ITEM_VIEW_MOVE_V1, CAPABILITY_ITEM_EQUIP_DROP_V1];
pub const STATE_DOMAIN_CONTAINER_VIEWS: u32 = 14;
pub const SNAPSHOT_TYPE_CONTAINER_VIEWS_V1: u32 = 1;
pub const DELTA_TYPE_CONTAINER_VIEWS_V1: u32 = 1;
pub const COMMAND_TYPE_CONTAINER_VIEW_INTENT: u32 = 21;

/// `BAGS0-RL-03`: open container views of one session (Canary's 4-bit container id).
pub const MAX_CONTAINER_VIEWS: usize = 16;
/// The largest view id; ids are `0..=15`.
pub const MAX_CONTAINER_VIEW_ID: u8 = (MAX_CONTAINER_VIEWS - 1) as u8;
/// `GAMEITEM01-CONTAINER-ENTRIES-MAX`: a container's capacity, and so the entries of one view.
pub const MAX_CONTAINER_VIEW_ENTRIES: usize = 20;
/// `ITEMV0-RL-03-CONTAINER-TREE`: live handles of a session that selected capability 14. The
/// capability 4 views ([`MAX_LIVE_ITEM_HANDLES`]) plus, for each of the 16 views, its container
/// (which may be visible nowhere else) and its 20 entries.
pub const MAX_LIVE_ITEM_HANDLES_CONTAINER_TREE: usize =
    MAX_LIVE_ITEM_HANDLES + MAX_CONTAINER_VIEWS * (1 + MAX_CONTAINER_VIEW_ENTRIES);

/// One `ItemEntryV1` as an element of a repeated field: its tag, a 1-byte length and the entry.
const MAX_ITEM_ENTRY_ELEMENT_BYTES: usize = 2 + MAX_ITEM_ENTRY_BYTES;
/// One `ContainerViewV1`, measured: view id 1 + 1, container 1 + 10, parent 1 + 10, capacity
/// 1 + 1 and 20 entry elements.
pub const MAX_CONTAINER_VIEW_BYTES: usize =
    2 + 2 * MAX_ITEM_TARGET_BYTES + 2 + MAX_CONTAINER_VIEW_ENTRIES * MAX_ITEM_ENTRY_ELEMENT_BYTES;
/// One `ContainerViewV1` as an element of a repeated field: its tag and a 2-byte length.
const MAX_CONTAINER_VIEW_ELEMENT_BYTES: usize = 3 + MAX_CONTAINER_VIEW_BYTES;
/// Domain 14 snapshot and delta: 16 view elements.
pub const MAX_CONTAINER_VIEWS_BYTES: usize = MAX_CONTAINER_VIEWS * MAX_CONTAINER_VIEW_ELEMENT_BYTES;
/// `ContainerOpenV1`: the handle 1 + 10 and `replace_view` 1 + 1.
const MAX_CONTAINER_OPEN_BYTES: usize = MAX_ITEM_TARGET_BYTES + 2;
/// `ContainerViewIntentV1`: the largest action, `open` 1 + 1 + 13 (`close` and `up` are 1 + 1 + 2).
pub const MAX_CONTAINER_VIEW_INTENT_BYTES: usize = 2 + MAX_CONTAINER_OPEN_BYTES;
/// `ContainerViewResultV1`: one small enum, with the same slack as the other results.
pub const MAX_CONTAINER_VIEW_RESULT_BYTES: usize = 4;

/// `ContainerViewV1`: one open view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerView {
    /// `0..=15`.
    pub view_id: u8,
    pub container_handle: ItemHandle,
    /// The container's parent when it is visible, for `UP`.
    pub parent_handle: Option<ItemHandle>,
    /// `1..=20`.
    pub capacity: u8,
    /// The direct entries in display order, at most `capacity`.
    pub entries: Vec<ItemEntry>,
}

/// `ContainerViewsV1`: every open view in ascending view id; the default means none is open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContainerViews {
    pub views: Vec<ContainerView>,
}

/// `ContainerViewIntentV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerViewIntent {
    /// Open `handle` in a new view, or in place of the open view `replace_view`.
    Open {
        handle: ItemHandle,
        replace_view: Option<u8>,
    },
    Close {
        view_id: u8,
    },
    /// Replace the view by its parent container.
    Up {
        view_id: u8,
    },
}

/// `ContainerViewOutcomeV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerViewOutcome {
    Opened = 1,
    Closed = 2,
    /// The handle or view no longer resolves, or the view has no visible parent.
    Stale = 3,
    TooFar = 4,
    NotAContainer = 5,
    /// A new view while 16 are open.
    TooManyViews = 6,
}

fn malformed<T>() -> WireResult<T> {
    Err(ItemViewWireError::Malformed)
}

fn nonzero_handle(value: u64) -> WireResult<ItemHandle> {
    ItemHandle::new(value).ok_or(ItemViewWireError::Malformed)
}

/// A view id above 15 is over `BAGS0-RL-03`.
fn view_id(value: u32) -> WireResult<u8> {
    u8::try_from(value)
        .ok()
        .filter(|id| *id <= MAX_CONTAINER_VIEW_ID)
        .ok_or(ItemViewWireError::LimitExceeded)
}

fn validate_view(view: &ContainerView) -> WireResult<()> {
    if view.view_id > MAX_CONTAINER_VIEW_ID
        || usize::from(view.capacity) > MAX_CONTAINER_VIEW_ENTRIES
        || view.entries.len() > MAX_CONTAINER_VIEW_ENTRIES
    {
        return Err(ItemViewWireError::LimitExceeded);
    }
    if view.capacity == 0 || view.entries.len() > usize::from(view.capacity) {
        return malformed();
    }
    // The container, its parent and its entries are distinct items.
    let mut seen = BTreeSet::new();
    let handles = std::iter::once(view.container_handle)
        .chain(view.parent_handle)
        .chain(view.entries.iter().map(|entry| entry.handle));
    for handle in handles {
        if !seen.insert(handle) {
            return malformed();
        }
    }
    Ok(())
}

fn validate_container_views(views: &ContainerViews) -> WireResult<()> {
    if views.views.len() > MAX_CONTAINER_VIEWS {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let (mut containers, mut entries) = (BTreeSet::new(), BTreeSet::new());
    for (index, view) in views.views.iter().enumerate() {
        validate_view(view)?;
        if index > 0 && views.views[index - 1].view_id >= view.view_id {
            return malformed();
        }
        // One view per container, and an item is the entry of one container only.
        if !containers.insert(view.container_handle)
            || view
                .entries
                .iter()
                .any(|entry| !entries.insert(entry.handle))
        {
            return malformed();
        }
    }
    Ok(())
}

fn encode_view(output: &mut Vec<u8>, view: &ContainerView) {
    let mut body = Vec::with_capacity(MAX_CONTAINER_VIEW_BYTES);
    push_nonzero_varint_field(&mut body, 1, u64::from(view.view_id));
    push_varint_field(&mut body, 2, view.container_handle.get());
    if let Some(parent) = view.parent_handle {
        push_varint_field(&mut body, 3, parent.get());
    }
    push_varint_field(&mut body, 4, u64::from(view.capacity));
    for entry in &view.entries {
        encode_entry(&mut body, 5, entry);
    }
    push_message_field(output, 1, &body);
}

fn decode_view(input: &[u8]) -> WireResult<ContainerView> {
    if input.len() > MAX_CONTAINER_VIEW_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut id, mut container, mut parent, mut capacity) = (None, None, None, None);
    let mut entries = Vec::new();
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 => set_once(&mut id, read_uint32(input, &mut cursor)?)?,
            0x10 => set_once(&mut container, read_varint(input, &mut cursor)?)?,
            0x18 => set_once(&mut parent, read_varint(input, &mut cursor)?)?,
            0x20 => set_once(&mut capacity, read_uint32(input, &mut cursor)?)?,
            0x2a => {
                if entries.len() == MAX_CONTAINER_VIEW_ENTRIES {
                    return Err(ItemViewWireError::LimitExceeded);
                }
                entries.push(decode_entry(read_bytes(input, &mut cursor)?)?);
            }
            _ => return malformed(),
        }
    }
    let capacity = capacity.unwrap_or(0);
    if capacity as usize > MAX_CONTAINER_VIEW_ENTRIES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let view = ContainerView {
        view_id: view_id(id.unwrap_or(0))?,
        container_handle: nonzero_handle(container.unwrap_or(0))?,
        // An absent parent is 0; a present zero is the same proto3 value.
        parent_handle: parent.and_then(ItemHandle::new),
        capacity: capacity as u8,
        entries,
    };
    validate_view(&view)?;
    Ok(view)
}

/// Domain 14, snapshot type 1 and delta type 1 (every open view).
pub fn encode_container_views(views: &ContainerViews) -> WireResult<Vec<u8>> {
    validate_container_views(views)?;
    let mut output = Vec::new();
    for view in &views.views {
        encode_view(&mut output, view);
    }
    Ok(output)
}

pub fn decode_container_views(payload: &[u8]) -> WireResult<ContainerViews> {
    if payload.len() > MAX_CONTAINER_VIEWS_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut views = ContainerViews::default();
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x0a => {
                if views.views.len() == MAX_CONTAINER_VIEWS {
                    return Err(ItemViewWireError::LimitExceeded);
                }
                views
                    .views
                    .push(decode_view(read_bytes(payload, &mut cursor)?)?);
            }
            _ => return malformed(),
        }
    }
    validate_container_views(&views)?;
    Ok(views)
}

/// `ClientCommand.payload` of command type 21. A view id above 15 is refused.
pub fn encode_container_view_intent(intent: &ContainerViewIntent) -> WireResult<Vec<u8>> {
    let mut output = Vec::with_capacity(MAX_CONTAINER_VIEW_INTENT_BYTES);
    match *intent {
        ContainerViewIntent::Open {
            handle,
            replace_view,
        } => {
            let mut body = Vec::with_capacity(MAX_CONTAINER_OPEN_BYTES);
            push_varint_field(&mut body, 1, handle.get());
            if let Some(view) = replace_view {
                view_id(u32::from(view))?;
                // Proto3 `optional`: present even at 0.
                push_varint_field(&mut body, 2, u64::from(view));
            }
            push_message_field(&mut output, 1, &body);
        }
        ContainerViewIntent::Close { view_id: view }
        | ContainerViewIntent::Up { view_id: view } => {
            view_id(u32::from(view))?;
            let mut body = Vec::with_capacity(2);
            push_nonzero_varint_field(&mut body, 1, u64::from(view));
            let field = if matches!(intent, ContainerViewIntent::Close { .. }) {
                2
            } else {
                3
            };
            push_message_field(&mut output, field, &body);
        }
    }
    Ok(output)
}

fn decode_open(input: &[u8]) -> WireResult<ContainerViewIntent> {
    let mut cursor = 0;
    let (mut handle, mut replace_view) = (None, None);
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 => set_once(&mut handle, read_varint(input, &mut cursor)?)?,
            0x10 => set_once(
                &mut replace_view,
                view_id(read_uint32(input, &mut cursor)?)?,
            )?,
            _ => return malformed(),
        }
    }
    Ok(ContainerViewIntent::Open {
        handle: nonzero_handle(handle.unwrap_or(0))?,
        replace_view,
    })
}

/// A missing or repeated action, a missing handle and any unknown field fail closed.
pub fn decode_container_view_intent(payload: &[u8]) -> WireResult<ContainerViewIntent> {
    if payload.len() > MAX_CONTAINER_VIEW_INTENT_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut intent = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let body = match key {
            0x0a | 0x12 | 0x1a => read_bytes(payload, &mut cursor)?,
            _ => return malformed(),
        };
        let action = match key {
            0x0a => decode_open(body)?,
            0x12 => ContainerViewIntent::Close {
                view_id: view_id(read_uint32_fields::<1>(body)?[0])?,
            },
            _ => ContainerViewIntent::Up {
                view_id: view_id(read_uint32_fields::<1>(body)?[0])?,
            },
        };
        set_once(&mut intent, action)?;
    }
    intent.ok_or(ItemViewWireError::Malformed)
}

/// `CommandResult.payload` of command type 21.
pub fn encode_container_view_result(outcome: ContainerViewOutcome) -> Vec<u8> {
    let mut output = Vec::with_capacity(2);
    push_varint_field(&mut output, 1, outcome as u64);
    output
}

pub fn decode_container_view_result(payload: &[u8]) -> WireResult<ContainerViewOutcome> {
    Ok(
        match read_result_enum(payload, MAX_CONTAINER_VIEW_RESULT_BYTES)? {
            1 => ContainerViewOutcome::Opened,
            2 => ContainerViewOutcome::Closed,
            3 => ContainerViewOutcome::Stale,
            4 => ContainerViewOutcome::TooFar,
            5 => ContainerViewOutcome::NotAContainer,
            6 => ContainerViewOutcome::TooManyViews,
            _ => return malformed(),
        },
    )
}

#[cfg(test)]
#[path = "container_tree_tests.rs"]
mod tests;
