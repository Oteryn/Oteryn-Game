//! Item view and item move codecs (ITEM-VIEW-1a, ITEM-EQUIP-WIRE-1).
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
//!
//! ITEM-EQUIP-WIRE-1 (ITEM-MOVE-WIRE-1 §3): capability 12 `ITEM_EQUIP_DROP_V1`, which requires
//! capability 4, adds the nine equipment slots to domain 9, the `EQUIPMENT {slot}` and
//! `GROUND {WorldTilePosition}` destinations to command type 9 and three results. The
//! `_with_equip_drop` codecs take the session's selection; without it they behave exactly as
//! ITEM-VIEW-1a, so domain 9 field 3, destination fields 3 and 4 and results 10 to 12 fail closed.
//!
//! BAGS-WIRE-1 (BAGS-0 §5): capability 14 `CONTAINER_TREE_V1` (`container_tree`) adds the
//! `CONTAINER {handle}` destination, field 5. The `_for` intent codecs take an
//! [`ItemMoveSelection`]; without capability 14 field 5 fails closed.

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
/// Registered capability `ITEM_EQUIP_DROP_V1` (ITEM-MOVE-WIRE-1 §3): extends command type 9 and
/// domain 9 of capability 4, so it owns no command type or domain of its own.
pub const CAPABILITY_ITEM_EQUIP_DROP_V1: u32 = 12;
pub const CAPABILITY_ITEM_EQUIP_DROP_V1_REQUIRES: &[u32] = &[CAPABILITY_ITEM_VIEW_MOVE_V1];

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
/// Equipment slots in domain 9 under capability 12: the nine non-container slots.
pub const MAX_EQUIPPED_ITEMS: usize = EquipmentSlot::ALL.len();
/// One `EquippedItemV1`, measured: the slot 1 + 1 and the item element.
pub const MAX_EQUIPPED_ITEM_BYTES: usize = 2 + MAX_ITEM_ENTRY_ELEMENT_BYTES;
/// One `EquippedItemV1` as an element of a repeated field.
const MAX_EQUIPPED_ITEM_ELEMENT_BYTES: usize = 2 + MAX_EQUIPPED_ITEM_BYTES;
/// Domain 9 snapshot and delta under capability 12: nine equipped elements and the other 21 of
/// the 30 items as entry elements.
pub const MAX_CHARACTER_INVENTORY_EQUIP_DROP_BYTES: usize = MAX_EQUIPPED_ITEMS
    * MAX_EQUIPPED_ITEM_ELEMENT_BYTES
    + (MAX_CHARACTER_INVENTORY_ITEMS - MAX_EQUIPPED_ITEMS) * MAX_ITEM_ENTRY_ELEMENT_BYTES;
/// Domain 11 snapshot and delta: the container handle 1 + 10 and 16 entry elements.
pub const MAX_OPEN_CONTAINER_BYTES: usize =
    11 + MAX_OPEN_CONTAINER_ENTRIES * MAX_ITEM_ENTRY_ELEMENT_BYTES;
/// `ItemTargetV1`: the handle 1 + 10.
pub const MAX_ITEM_TARGET_BYTES: usize = 11;
/// `ItemMoveIntentV1`: the source handle 1 + 10 and the empty `main_backpack` destination 1 + 1.
pub const MAX_ITEM_MOVE_INTENT_BYTES: usize = 13;
/// `WorldTilePositionV1`, measured: x 1 + 5, y 1 + 5, floor (int16 range) 1 + 3.
pub const MAX_WORLD_TILE_POSITION_BYTES: usize = 16;
/// `ItemMoveIntentV1` under capability 12: the source handle 1 + 10 and the largest destination,
/// `ground` 1 + 1 + 16 (`equipment` is 1 + 1 + 2).
pub const MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES: usize = 11 + 2 + MAX_WORLD_TILE_POSITION_BYTES;
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

/// `CharacterInventoryV1`: the main backpack slot and its direct entries in display order, and
/// under capability 12 the occupied equipment slots in ascending slot order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterInventory {
    pub main_backpack: Option<ItemEntry>,
    pub entries: Vec<ItemEntry>,
    pub equipment: Vec<EquippedItem>,
}

/// `OpenContainerV1`: the one open corpse and its entries; the default means nothing is open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenContainer {
    pub container_handle: Option<ItemHandle>,
    pub entries: Vec<ItemEntry>,
}

/// `EquipmentSlotV1`: the nine non-container slots (ITEM-MOVE-WIRE-1 §3); `UNSPECIFIED` (0) and
/// unknown values fail closed and the container slot is not a destination.
///
/// These wire values are never durable slot keys: storage uses the GAME-ITEM-01 §6.1 semantic
/// slot keys, mapped by the separate [`EQUIPMENT_SLOT_SEMANTIC_KEYS`] table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EquipmentSlot {
    Head = 1,
    Necklace = 2,
    Armor = 3,
    RightHand = 4,
    LeftHand = 5,
    Legs = 6,
    Feet = 7,
    Ring = 8,
    Ammo = 9,
}

impl EquipmentSlot {
    pub const ALL: [Self; 9] = [
        Self::Head,
        Self::Necklace,
        Self::Armor,
        Self::RightHand,
        Self::LeftHand,
        Self::Legs,
        Self::Feet,
        Self::Ring,
        Self::Ammo,
    ];

    fn from_wire(value: u32) -> WireResult<Self> {
        Self::ALL
            .into_iter()
            .find(|slot| *slot as u32 == value)
            .ok_or(ItemViewWireError::Malformed)
    }

    /// The GAME-ITEM-01 §6.1 semantic slot key that durable rows use for this slot.
    #[must_use]
    pub fn semantic_key(self) -> &'static str {
        EQUIPMENT_SLOT_SEMANTIC_KEYS
            .iter()
            .find(|(slot, _)| *slot == self)
            .map_or_else(
                || unreachable!("every slot has a semantic key"),
                |(_, key)| *key,
            )
    }
}

/// The wire slot to GAME-ITEM-01 §6.1 semantic slot key table. It is kept apart from the enum
/// values so that a wire renumbering can never change a durable key.
pub const EQUIPMENT_SLOT_SEMANTIC_KEYS: [(EquipmentSlot, &str); 9] = [
    (EquipmentSlot::Head, "HEAD"),
    (EquipmentSlot::Necklace, "AMULET"),
    (EquipmentSlot::Armor, "TORSO"),
    (EquipmentSlot::RightHand, "WEAPON"),
    (EquipmentSlot::LeftHand, "SHIELD"),
    (EquipmentSlot::Legs, "LEGS"),
    (EquipmentSlot::Feet, "FEET"),
    (EquipmentSlot::Ring, "RING"),
    (EquipmentSlot::Ammo, "EXTRA"),
];

/// `EquippedItemV1`: an occupied slot of domain 9 under capability 12; an empty slot is absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquippedItem {
    pub slot: EquipmentSlot,
    pub item: ItemEntry,
}

/// `WorldTilePositionV1`: the pinned frame's native coordinates; the floor is in the int16 range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldTilePosition {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

/// The `ItemMoveIntentV1.destination` oneof. `Equipment` and `Ground` need capability 12,
/// `Container` capability 14.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemMoveDestination {
    MainBackpack,
    Equipment(EquipmentSlot),
    Ground(WorldTilePosition),
    /// Into the named visible container (BAGS-0 §5).
    Container(ItemHandle),
}

/// The capabilities of a session's selection that extend command type 9.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemMoveSelection {
    /// Capability 12: the `equipment` and `ground` destinations.
    pub equip_drop: bool,
    /// Capability 14: the `container` destination.
    pub container_tree: bool,
}

/// `ItemMoveIntentV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemMoveIntent {
    pub source: ItemHandle,
    pub destination: ItemMoveDestination,
}

/// `ItemMoveOutcomeV1`, the WIRE-0 §5 results; 10 to 12 (ITEM-MOVE-WIRE-1 §3) need capability 12.
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
    /// The wrong slot, a hands conflict, a container or unknown equipment semantics.
    SlotMismatch = 10,
    RequirementNotMet = 11,
    /// The tile does not accept the item, no line of sight, or a limit is reached.
    Blocked = 12,
}

impl ItemMoveOutcome {
    fn needs_equip_drop(self) -> bool {
        matches!(
            self,
            Self::SlotMismatch | Self::RequirementNotMet | Self::Blocked
        )
    }
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

pub(crate) fn encode_entry(output: &mut Vec<u8>, field: u64, entry: &ItemEntry) {
    let mut body = Vec::with_capacity(MAX_ITEM_ENTRY_BYTES);
    push_varint_field(&mut body, 1, entry.handle.get());
    push_varint_field(&mut body, 2, u64::from(entry.item_definition_ref.get()));
    push_varint_field(&mut body, 3, u64::from(entry.count.get()));
    push_nonzero_varint_field(&mut body, 4, u64::from(entry.sub_type));
    push_message_field(output, field, &body);
}

pub(crate) fn decode_entry(input: &[u8]) -> WireResult<ItemEntry> {
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

fn validate_character_inventory(view: &CharacterInventory, equip_drop: bool) -> WireResult<()> {
    if !equip_drop && !view.equipment.is_empty() {
        return malformed();
    }
    if usize::from(view.main_backpack.is_some()) + view.entries.len() + view.equipment.len()
        > MAX_CHARACTER_INVENTORY_ITEMS
    {
        return Err(ItemViewWireError::LimitExceeded);
    }
    if view.main_backpack.is_none() && !view.entries.is_empty() {
        return malformed();
    }
    // One item per slot, in ascending slot order.
    if view
        .equipment
        .windows(2)
        .any(|pair| pair[0].slot >= pair[1].slot)
    {
        return malformed();
    }
    unique_handles(
        view.main_backpack.map(|backpack| backpack.handle),
        view.entries
            .iter()
            .chain(view.equipment.iter().map(|equipped| &equipped.item)),
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

fn decode_equipped_item(input: &[u8]) -> WireResult<EquippedItem> {
    if input.len() > MAX_EQUIPPED_ITEM_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut slot, mut item) = (None, None);
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 => set_once(&mut slot, read_uint32(input, &mut cursor)?)?,
            0x12 => set_once(&mut item, decode_entry(read_bytes(input, &mut cursor)?)?)?,
            _ => return malformed(),
        }
    }
    Ok(EquippedItem {
        slot: EquipmentSlot::from_wire(slot.unwrap_or(0))?,
        item: item.ok_or(ItemViewWireError::Malformed)?,
    })
}

/// Domain 9, snapshot type 1 and delta type 1 (the whole view), without capability 12.
pub fn encode_character_inventory(view: &CharacterInventory) -> WireResult<Vec<u8>> {
    encode_character_inventory_with_equip_drop(view, false)
}

/// Domain 9 for a session that did (`equip_drop`) or did not select capability 12; without it a
/// view with equipment is refused.
pub fn encode_character_inventory_with_equip_drop(
    view: &CharacterInventory,
    equip_drop: bool,
) -> WireResult<Vec<u8>> {
    validate_character_inventory(view, equip_drop)?;
    let mut output = Vec::new();
    if let Some(backpack) = &view.main_backpack {
        encode_entry(&mut output, 1, backpack);
    }
    for entry in &view.entries {
        encode_entry(&mut output, 2, entry);
    }
    for equipped in &view.equipment {
        let mut body = Vec::with_capacity(MAX_EQUIPPED_ITEM_BYTES);
        push_varint_field(&mut body, 1, equipped.slot as u64);
        encode_entry(&mut body, 2, &equipped.item);
        push_message_field(&mut output, 3, &body);
    }
    Ok(output)
}

pub fn decode_character_inventory(payload: &[u8]) -> WireResult<CharacterInventory> {
    decode_character_inventory_with_equip_drop(payload, false)
}

/// Without `equip_drop`, field 3 fails closed and the ITEM-VIEW-1a bound applies.
pub fn decode_character_inventory_with_equip_drop(
    payload: &[u8],
    equip_drop: bool,
) -> WireResult<CharacterInventory> {
    let bound = if equip_drop {
        MAX_CHARACTER_INVENTORY_EQUIP_DROP_BYTES
    } else {
        MAX_CHARACTER_INVENTORY_BYTES
    };
    if payload.len() > bound {
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
            0x1a if equip_drop => {
                let equipped = decode_equipped_item(read_bytes(payload, &mut cursor)?)?;
                // Ascending slots, so at most nine.
                if view
                    .equipment
                    .last()
                    .is_some_and(|previous| previous.slot >= equipped.slot)
                {
                    return malformed();
                }
                view.equipment.push(equipped);
            }
            _ => return malformed(),
        }
    }
    validate_character_inventory(&view, equip_drop)?;
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

fn push_sint32(output: &mut Vec<u8>, field: u64, value: i32) {
    let zigzag = ((value << 1) ^ (value >> 31)) as u32;
    push_nonzero_varint_field(output, field, u64::from(zigzag));
}

fn decode_sint32(value: u64) -> WireResult<i32> {
    let zigzag = u32::try_from(value).map_err(|_| ItemViewWireError::Malformed)?;
    Ok(((zigzag >> 1) as i32) ^ -((zigzag & 1) as i32))
}

fn encode_position(position: WorldTilePosition) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_WORLD_TILE_POSITION_BYTES);
    push_sint32(&mut output, 1, position.x);
    push_sint32(&mut output, 2, position.y);
    push_sint32(&mut output, 3, i32::from(position.floor));
    output
}

/// Absent coordinates are 0 (proto3); a floor outside the int16 range fails closed.
fn decode_position(input: &[u8]) -> WireResult<WorldTilePosition> {
    if input.len() > MAX_WORLD_TILE_POSITION_BYTES {
        return Err(ItemViewWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut x, mut y, mut floor) = (None, None, None);
    while cursor < input.len() {
        let field = match read_varint(input, &mut cursor)? {
            0x08 => &mut x,
            0x10 => &mut y,
            0x18 => &mut floor,
            _ => return malformed(),
        };
        set_once(field, decode_sint32(read_varint(input, &mut cursor)?)?)?;
    }
    Ok(WorldTilePosition {
        x: x.unwrap_or(0),
        y: y.unwrap_or(0),
        floor: i16::try_from(floor.unwrap_or(0)).map_err(|_| ItemViewWireError::Malformed)?,
    })
}

/// `EquipmentDestinationV1`: the slot is required; `UNSPECIFIED` and unknown values fail closed.
fn decode_equipment_destination(input: &[u8]) -> WireResult<EquipmentSlot> {
    let mut cursor = 0;
    let mut slot = None;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 => set_once(&mut slot, read_uint32(input, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    EquipmentSlot::from_wire(slot.unwrap_or(0))
}

/// `ClientCommand.payload` of command type 9, without capability 12.
pub fn encode_item_move_intent(intent: &ItemMoveIntent) -> WireResult<Vec<u8>> {
    encode_item_move_intent_with_equip_drop(intent, false)
}

/// Without `equip_drop` the `equipment` and `ground` destinations are refused.
pub fn encode_item_move_intent_with_equip_drop(
    intent: &ItemMoveIntent,
    equip_drop: bool,
) -> WireResult<Vec<u8>> {
    encode_item_move_intent_for(
        intent,
        ItemMoveSelection {
            equip_drop,
            container_tree: false,
        },
    )
}

/// A destination the selection does not admit is refused.
pub fn encode_item_move_intent_for(
    intent: &ItemMoveIntent,
    selection: ItemMoveSelection,
) -> WireResult<Vec<u8>> {
    let mut output = Vec::with_capacity(MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES);
    push_varint_field(&mut output, 1, intent.source.get());
    match intent.destination {
        ItemMoveDestination::MainBackpack => push_message_field(&mut output, 2, &[]),
        ItemMoveDestination::Container(handle) if selection.container_tree => {
            let mut body = Vec::with_capacity(MAX_ITEM_TARGET_BYTES);
            push_varint_field(&mut body, 1, handle.get());
            push_message_field(&mut output, 5, &body);
        }
        ItemMoveDestination::Container(_) => return malformed(),
        _ if !selection.equip_drop => return malformed(),
        ItemMoveDestination::Equipment(slot) => {
            let mut body = Vec::with_capacity(2);
            push_varint_field(&mut body, 1, slot as u64);
            push_message_field(&mut output, 3, &body);
        }
        ItemMoveDestination::Ground(position) => {
            push_message_field(&mut output, 4, &encode_position(position));
        }
    }
    Ok(output)
}

/// A missing source handle or destination, a non-empty `main_backpack` body and any other field
/// (the destinations capability 12 adds) fail closed.
pub fn decode_item_move_intent(payload: &[u8]) -> WireResult<ItemMoveIntent> {
    decode_item_move_intent_with_equip_drop(payload, false)
}

/// With `equip_drop`, fields 3 `equipment` and 4 `ground` are destinations too, and at most one
/// destination is present.
pub fn decode_item_move_intent_with_equip_drop(
    payload: &[u8],
    equip_drop: bool,
) -> WireResult<ItemMoveIntent> {
    decode_item_move_intent_for(
        payload,
        ItemMoveSelection {
            equip_drop,
            container_tree: false,
        },
    )
}

/// With `container_tree`, field 5 `container` is a destination too (its handle is required). The
/// capability 12 bound covers it, so a selection with capability 14 keeps 29 bytes.
pub fn decode_item_move_intent_for(
    payload: &[u8],
    selection: ItemMoveSelection,
) -> WireResult<ItemMoveIntent> {
    let ItemMoveSelection {
        equip_drop,
        container_tree,
    } = selection;
    let bound = if equip_drop || container_tree {
        MAX_ITEM_MOVE_INTENT_EQUIP_DROP_BYTES
    } else {
        MAX_ITEM_MOVE_INTENT_BYTES
    };
    if payload.len() > bound {
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
            0x1a if equip_drop => {
                let slot = decode_equipment_destination(read_bytes(payload, &mut cursor)?)?;
                set_once(&mut destination, ItemMoveDestination::Equipment(slot))?;
            }
            0x22 if equip_drop => {
                let position = decode_position(read_bytes(payload, &mut cursor)?)?;
                set_once(&mut destination, ItemMoveDestination::Ground(position))?;
            }
            0x2a if container_tree => {
                let handle = decode_item_target(read_bytes(payload, &mut cursor)?)?;
                set_once(&mut destination, ItemMoveDestination::Container(handle))?;
            }
            _ => return malformed(),
        }
    }
    Ok(ItemMoveIntent {
        source: nonzero_u64(source.unwrap_or(0))?,
        destination: destination.ok_or(ItemViewWireError::Malformed)?,
    })
}

/// `CommandResult.payload` of command type 9, without capability 12.
pub fn encode_item_move_result(outcome: ItemMoveOutcome) -> WireResult<Vec<u8>> {
    encode_item_move_result_with_equip_drop(outcome, false)
}

/// Without `equip_drop` the results 10 to 12 are refused.
pub fn encode_item_move_result_with_equip_drop(
    outcome: ItemMoveOutcome,
    equip_drop: bool,
) -> WireResult<Vec<u8>> {
    if !equip_drop && outcome.needs_equip_drop() {
        return malformed();
    }
    let mut output = Vec::with_capacity(2);
    push_varint_field(&mut output, 1, outcome as u64);
    Ok(output)
}

pub fn decode_item_move_result(payload: &[u8]) -> WireResult<ItemMoveOutcome> {
    decode_item_move_result_with_equip_drop(payload, false)
}

/// Without `equip_drop` the results 10 to 12 fail closed as unknown.
pub fn decode_item_move_result_with_equip_drop(
    payload: &[u8],
    equip_drop: bool,
) -> WireResult<ItemMoveOutcome> {
    let outcome = match read_result_enum(payload, MAX_ITEM_MOVE_RESULT_BYTES)? {
        1 => ItemMoveOutcome::Moved,
        2 => ItemMoveOutcome::Stale,
        3 => ItemMoveOutcome::TooFar,
        4 => ItemMoveOutcome::NoBackpack,
        5 => ItemMoveOutcome::NoRoom,
        6 => ItemMoveOutcome::NotOwner,
        7 => ItemMoveOutcome::NotPickupable,
        8 => ItemMoveOutcome::NotSupported,
        9 => ItemMoveOutcome::Rejected,
        10 => ItemMoveOutcome::SlotMismatch,
        11 => ItemMoveOutcome::RequirementNotMet,
        12 => ItemMoveOutcome::Blocked,
        _ => return malformed(),
    };
    if !equip_drop && outcome.needs_equip_drop() {
        return malformed();
    }
    Ok(outcome)
}

#[cfg(test)]
#[path = "item_view_tests.rs"]
mod tests;
