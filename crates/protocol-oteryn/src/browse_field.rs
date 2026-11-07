//! Browse-field commands and the selected extension of container-view domain 14.
//!
//! Crystal `201189f`, `container.cpp:55-84` supplies the thirty-entry page;
//! `protocolgame.cpp:3817-3826,5784-5793` supplies native positions and u16 page indexes/totals.
//! Views and their handles are session-local. Item moves retain command 9 and its durable owner.

use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, read_bytes, read_uint32,
    read_uint32_fields, read_varint, set_once,
};
use crate::container_tree::{MAX_CONTAINER_VIEW_ID, MAX_CONTAINER_VIEWS};
use crate::item_view::{
    ItemViewWireError, MAX_ITEM_ENTRY_BYTES, MAX_ITEM_TARGET_BYTES, MAX_LIVE_ITEM_HANDLES,
    WorldTilePosition, decode_sint32, push_sint32,
};
use crate::world_map::{BOTTOM_FLOOR, TOP_FLOOR};

pub const CAPABILITY_BROWSE_FIELD_V1: u32 = 230;
pub const COMMAND_TYPE_BROWSE_FIELD_INTENT: u32 = 440;
pub const BROWSE_FIELD_CAPACITY: u8 = 30;
pub const MAX_BROWSE_FIELD_PAGE_ENTRIES: usize = BROWSE_FIELD_CAPACITY as usize;
/// Two zigzag uint16 coordinates (four bytes each) and native floor -15..=0 (two bytes).
const MAX_BROWSE_FIELD_POSITION_BYTES: usize = 10;
/// One Open position message, or the smaller Seek view/index message.
pub const MAX_BROWSE_FIELD_INTENT_BYTES: usize = 2 + MAX_BROWSE_FIELD_POSITION_BYTES;
/// Native coordinates plus a u16 page index and a u16 item total (four bytes each).
pub(crate) const MAX_BROWSE_FIELD_PAGE_BYTES: usize = MAX_BROWSE_FIELD_POSITION_BYTES + 8;
/// View id, synthetic handle, capacity, thirty entries and the page metadata message.
pub const MAX_BROWSE_FIELD_VIEW_BYTES: usize = 2
    + MAX_ITEM_TARGET_BYTES
    + 2
    + MAX_BROWSE_FIELD_PAGE_ENTRIES * (2 + MAX_ITEM_ENTRY_BYTES)
    + 2
    + MAX_BROWSE_FIELD_PAGE_BYTES;
/// The same sixteen view slots as ordinary containers, under capability 230 selection.
pub const MAX_BROWSE_FIELD_VIEWS_BYTES: usize =
    MAX_CONTAINER_VIEWS * (3 + MAX_BROWSE_FIELD_VIEW_BYTES);
/// Existing session views plus sixteen synthetic handles and thirty entries per page.
pub const MAX_LIVE_ITEM_HANDLES_BROWSE_FIELD: usize =
    MAX_LIVE_ITEM_HANDLES + MAX_CONTAINER_VIEWS * (1 + MAX_BROWSE_FIELD_PAGE_ENTRIES);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseFieldIntent {
    Open { position: WorldTilePosition },
    Seek { view_id: u8, index: u16 },
}

/// Present only on a virtual tile view, and only when capability 230 was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowseFieldPage {
    pub position: WorldTilePosition,
    pub index: u16,
    pub total: u16,
}

fn encode_coordinates(position: WorldTilePosition, output: &mut Vec<u8>) {
    push_sint32(output, 1, position.x);
    push_sint32(output, 2, position.y);
    push_sint32(output, 3, i32::from(position.floor));
}

pub(crate) fn validate_browse_position(position: WorldTilePosition) -> WireResult<()> {
    // The bundle uses u16 x/y and native floor -z, matching the existing map capability.
    if u16::try_from(position.x).is_err()
        || u16::try_from(position.y).is_err()
        || !(BOTTOM_FLOOR..=TOP_FLOOR).contains(&position.floor)
    {
        return Err(ItemViewWireError::Malformed);
    }
    Ok(())
}

fn position(values: [i32; 3]) -> WireResult<WorldTilePosition> {
    let position = WorldTilePosition {
        x: values[0],
        y: values[1],
        floor: i16::try_from(values[2]).map_err(|_| ItemViewWireError::Malformed)?,
    };
    validate_browse_position(position)?;
    Ok(position)
}

/// Command 440. Seek alignment and the current item total are checked by the view owner.
pub fn encode_browse_field_intent(intent: &BrowseFieldIntent) -> WireResult<Vec<u8>> {
    let mut output = Vec::with_capacity(MAX_BROWSE_FIELD_INTENT_BYTES);
    let mut body = Vec::new();
    match *intent {
        BrowseFieldIntent::Open { position } => {
            validate_browse_position(position)?;
            encode_coordinates(position, &mut body);
            push_message_field(&mut output, 1, &body);
        }
        BrowseFieldIntent::Seek { view_id, index } => {
            if view_id > MAX_CONTAINER_VIEW_ID {
                return Err(ItemViewWireError::LimitExceeded);
            }
            push_nonzero_varint_field(&mut body, 1, u64::from(view_id));
            push_nonzero_varint_field(&mut body, 2, u64::from(index));
            push_message_field(&mut output, 2, &body);
        }
    }
    Ok(output)
}

pub fn decode_browse_field_intent(payload: &[u8]) -> WireResult<BrowseFieldIntent> {
    if payload.len() > MAX_BROWSE_FIELD_INTENT_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut intent = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let body = match key {
            0x0a | 0x12 => read_bytes(payload, &mut cursor)?,
            _ => return Err(ItemViewWireError::Malformed),
        };
        let action = if key == 0x0a {
            let values = read_uint32_fields::<3>(body)?;
            BrowseFieldIntent::Open {
                position: position([
                    decode_sint32(u64::from(values[0]))?,
                    decode_sint32(u64::from(values[1]))?,
                    decode_sint32(u64::from(values[2]))?,
                ])?,
            }
        } else {
            let [view, index] = read_uint32_fields::<2>(body)?;
            let view_id = u8::try_from(view)
                .ok()
                .filter(|id| *id <= MAX_CONTAINER_VIEW_ID)
                .ok_or(ItemViewWireError::LimitExceeded)?;
            BrowseFieldIntent::Seek {
                view_id,
                index: u16::try_from(index).map_err(|_| ItemViewWireError::LimitExceeded)?,
            }
        };
        set_once(&mut intent, action)?;
    }
    intent.ok_or(ItemViewWireError::Malformed)
}

pub(crate) fn encode_page(page: BrowseFieldPage) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_BROWSE_FIELD_PAGE_BYTES);
    encode_coordinates(page.position, &mut output);
    push_nonzero_varint_field(&mut output, 4, u64::from(page.index));
    push_nonzero_varint_field(&mut output, 5, u64::from(page.total));
    output
}

pub(crate) fn decode_page(payload: &[u8]) -> WireResult<BrowseFieldPage> {
    if payload.len() > MAX_BROWSE_FIELD_PAGE_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut x, mut y, mut floor, mut index, mut total) = (None, None, None, None, None);
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x08 => set_once(&mut x, decode_sint32(read_varint(payload, &mut cursor)?)?)?,
            0x10 => set_once(&mut y, decode_sint32(read_varint(payload, &mut cursor)?)?)?,
            0x18 => set_once(
                &mut floor,
                decode_sint32(read_varint(payload, &mut cursor)?)?,
            )?,
            0x20 => set_once(&mut index, read_uint32(payload, &mut cursor)?)?,
            0x28 => set_once(&mut total, read_uint32(payload, &mut cursor)?)?,
            _ => return Err(ItemViewWireError::Malformed),
        }
    }
    Ok(BrowseFieldPage {
        position: position([x.unwrap_or(0), y.unwrap_or(0), floor.unwrap_or(0)])?,
        index: u16::try_from(index.unwrap_or(0)).map_err(|_| ItemViewWireError::LimitExceeded)?,
        total: u16::try_from(total.unwrap_or(0)).map_err(|_| ItemViewWireError::LimitExceeded)?,
    })
}

#[cfg(test)]
#[path = "browse_field_tests.rs"]
mod tests;
