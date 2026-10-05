//! World map view codec (MAP-WIRE-2).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/world_map_v1.proto`, from the MAP-WIRE-1 contract
//! §3 (`candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md`). Capability 18
//! `WORLD_MAP_VIEW_V1` (requires capabilities 6 and 4) gates state domain 17 `WORLD_MAP_VIEW`:
//! the composed stack of every tile in the 18x14 window over every floor in view. The capability
//! stays `offered: false` until MAP-CUTOVER-1 boots a World from a bundle.
//!
//! Decoding is strict, and encoding refuses the same values before any byte is emitted: unknown,
//! repeated or zero-valued required fields, a tile outside the window, unsorted or duplicate
//! tiles, a base ordinal of 64 or more, a zero handle, a `display_only` that is not `true`, not
//! exactly one origin or definition reference, an `object_revision` on an entry without a base
//! ordinal, a count outside `1..=100`, a ground speed above 1,000, an appearance above 65,535, a
//! `first_visible_floor` outside the floors the origin can draw, and every count or byte size
//! over its bound all fail closed. A delta is decoded against the view it applies to: another
//! binding is [`WorldMapWireError::BindingMismatch`] (`STATE_REVISION_MISMATCH`, the client
//! resyncs), and an origin that is not the view's origin or one step from it on the same floor
//! fails closed.

// The client-side decoders are exercised by the tests until MAP-CLIENT-1.
#![cfg_attr(not(test), allow(dead_code))]

use std::num::{NonZeroU32, NonZeroU64};

use crate::charm_wire::{push_varint, read_bytes, read_varint};
use crate::world_spatial::ActorPosition;

/// Registered capability `WORLD_MAP_VIEW_V1`: state domain 17.
pub const CAPABILITY_WORLD_MAP_VIEW_V1: u32 = 18;
/// Capability 18 requires `WORLD_SPATIAL_ENTITIES` (6) and `ITEM_VIEW_MOVE_V1` (4): item handles
/// and `ItemTargetV1` are capability-4 types.
pub const CAPABILITY_WORLD_MAP_VIEW_REQUIRES: [u32; 2] = [4, 6];
/// Registered state domain `WORLD_MAP_VIEW`, owned by the current ChannelRuntime.
pub const STATE_DOMAIN_WORLD_MAP_VIEW: u32 = 17;
pub const SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1: u32 = 1;
pub const DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1: u32 = 1;

/// `MAPW-RL-01`: item entries carried per tile.
pub const MAX_TILE_ITEMS: usize = 10;
/// `MAPW-RL-02`: tiles per snapshot, 18 x 14 x 8.
pub const MAX_SNAPSHOT_TILES: usize = 2_016;
/// `MAPW-RL-03`: tiles and cleared entries together per delta.
pub const MAX_DELTA_ENTRIES: usize = 248;
/// `MAPW-RL-04`: handle-bearing entries in one map view.
pub const MAX_MAP_VIEW_HANDLES: usize = 1_024;

/// Encoded bound of one `MapItemV1`, without its field tag and length.
pub const MAX_MAP_ITEM_BYTES: usize = 32;
/// Encoded bound of one `MapTileV1`, with its field tag and length.
pub const MAX_MAP_TILE_BYTES: usize = 360;
/// Encoded bound of the header with its field tag and length, with world coordinates in
/// `0..=65,535`.
pub const MAX_MAP_HEADER_BYTES: usize = 95;
/// `MAPW-RL-02` x `MAX_MAP_TILE_BYTES` + 128.
pub const MAX_SNAPSHOT_PAYLOAD_BYTES: usize = MAX_SNAPSHOT_TILES * MAX_MAP_TILE_BYTES + 128;
/// `MAPW-RL-03` x (`MAX_MAP_TILE_BYTES` + 16) + 128.
pub const MAX_DELTA_PAYLOAD_BYTES: usize =
    MAX_DELTA_ENTRIES * MAX_MAP_TILE_BYTES + MAX_DELTA_ENTRIES * 16 + 128;

/// The highest base ordinal (the bundle placement key keeps 8 bits, format §7 bounds a stack).
pub const MAX_BASE_ORDINAL: u8 = 63;
pub const MAX_COUNT: u32 = 100;
pub const MAX_GROUND_SPEED: u16 = 1_000;
pub const MAX_APPEARANCE_ID: u32 = 65_535;

/// The window: 18x14 tiles from `(x - 8, y - 6)` to `(x + 9, y + 7)`.
pub const VIEW_WIDTH: i32 = 18;
pub const VIEW_HEIGHT: i32 = 14;
pub const VIEW_LEFT: i32 = 8;
pub const VIEW_TOP: i32 = 6;
/// The surface native floor; floors `-7..=0` are on or above it.
pub const SURFACE_FLOOR: i16 = -7;
/// The highest and lowest native floors.
pub const TOP_FLOOR: i16 = 0;
pub const BOTTOM_FLOOR: i16 = -15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldMapWireError {
    /// Not a valid encoding of the schema, or a value outside its declared domain.
    Malformed,
    /// The payload is over its byte bound or a repeated field over its registered limit.
    LimitExceeded,
    /// A delta whose `(content_generation, bundle_digest, reset_epoch)` differs from the view's:
    /// `STATE_REVISION_MISMATCH`, and the client resyncs.
    BindingMismatch,
}

type Result<T> = std::result::Result<T, WorldMapWireError>;

const fn malformed<T>() -> Result<T> {
    Err(WorldMapWireError::Malformed)
}

/// Exactly one, non-zero definition reference (contract §3 Definition reference).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapDefinition {
    /// `item_definition_ref` (1): the WorldSpatialEntityV1 item reference space.
    Item(NonZeroU32),
    /// `terrain_definition_ref` (8): the bundle palette compact id of a Terrain record.
    Terrain(NonZeroU32),
}

/// Exactly one origin (contract §3, §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapOrigin {
    /// An unhidden base entry with no handle, with its overlay revision (0 if none).
    BaseOrdinal { ordinal: u8, object_revision: u64 },
    /// A handle-bearing entry within the handle budget.
    Handle(NonZeroU64),
    /// A handle-bearing entry beyond the handle budget.
    DisplayOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapItem {
    pub definition: MapDefinition,
    /// `1..=100`.
    pub count: u32,
    pub sub_type: u32,
    pub origin: MapOrigin,
    /// The 15.30 appearance object id, 0 if none.
    pub appearance_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTile {
    pub position: ActorPosition,
    /// `1..=10`, stack order, ground first.
    pub items: Vec<MapItem>,
    pub more: bool,
    /// `0..=1,000`; 0 without a ground.
    pub ground_speed: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapViewHeader {
    pub content_generation: [u8; 32],
    pub bundle_digest: [u8; 32],
    pub reset_epoch: u64,
    pub origin: ActorPosition,
    pub first_visible_floor: i16,
}

impl MapViewHeader {
    fn same_binding(&self, other: &Self) -> bool {
        self.content_generation == other.content_generation
            && self.bundle_digest == other.bundle_digest
            && self.reset_epoch == other.reset_epoch
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMapViewSnapshot {
    pub header: MapViewHeader,
    /// Strictly ascending by `(floor, y, x)`, each in the window.
    pub tiles: Vec<MapTile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMapViewDelta {
    pub header: MapViewHeader,
    pub tiles: Vec<MapTile>,
    pub cleared: Vec<ActorPosition>,
}

/// The floors in view at an actor floor: `-7..=0` on or above the surface, else two above and
/// two below, bounded to -15. `None` outside `-15..=0`.
pub fn floors_in_view(floor: i16) -> Option<std::ops::RangeInclusive<i16>> {
    if !(BOTTOM_FLOOR..=TOP_FLOOR).contains(&floor) {
        return None;
    }
    Some(if floor >= SURFACE_FLOOR {
        SURFACE_FLOOR..=TOP_FLOOR
    } else {
        (floor - 2).max(BOTTOM_FLOOR)..=floor + 2
    })
}

/// The start value of the visible-floor rule: 0 on or above the surface, else
/// `min(floor + 2, -8)`. It is also the highest `first_visible_floor` the floor admits.
pub fn first_visible_floor_start(floor: i16) -> i16 {
    if floor >= SURFACE_FLOOR {
        TOP_FLOOR
    } else {
        (floor + 2).min(SURFACE_FLOOR - 1)
    }
}

/// Whether `position` lies in the window of `origin`, with each floor shifted by its floor
/// difference to the origin (the Tibia perspective).
pub fn in_window(origin: ActorPosition, position: ActorPosition) -> bool {
    let Some(floors) = floors_in_view(origin.floor) else {
        return false;
    };
    if !floors.contains(&position.floor) {
        return false;
    }
    let shift = i64::from(position.floor) - i64::from(origin.floor);
    let left = i64::from(origin.x) - i64::from(VIEW_LEFT) + shift;
    let top = i64::from(origin.y) - i64::from(VIEW_TOP) + shift;
    let (x, y) = (i64::from(position.x), i64::from(position.y));
    (left..left + i64::from(VIEW_WIDTH)).contains(&x)
        && (top..top + i64::from(VIEW_HEIGHT)).contains(&y)
}

/// The wire order of tiles and cleared entries: `(floor, y, x)`.
pub fn tile_order(position: &ActorPosition) -> (i16, i32, i32) {
    (position.floor, position.y, position.x)
}

fn check_header(header: &MapViewHeader) -> Result<()> {
    let floor = header.origin.floor;
    if floors_in_view(floor).is_none() {
        return malformed();
    }
    let visible = header.first_visible_floor;
    if visible < floor || visible > first_visible_floor_start(floor) {
        return malformed();
    }
    Ok(())
}

fn check_item(item: &MapItem) -> Result<()> {
    if !(1..=MAX_COUNT).contains(&item.count) {
        return malformed();
    }
    if let MapOrigin::BaseOrdinal { ordinal, .. } = item.origin
        && ordinal > MAX_BASE_ORDINAL
    {
        return malformed();
    }
    Ok(())
}

fn check_tile(origin: ActorPosition, tile: &MapTile) -> Result<()> {
    if tile.items.is_empty() {
        return malformed();
    }
    if tile.items.len() > MAX_TILE_ITEMS {
        return Err(WorldMapWireError::LimitExceeded);
    }
    if tile.ground_speed > MAX_GROUND_SPEED || !in_window(origin, tile.position) {
        return malformed();
    }
    tile.items.iter().try_for_each(check_item)
}

fn check_ascending<'a>(positions: impl Iterator<Item = &'a ActorPosition>) -> Result<()> {
    let mut last = None;
    for position in positions {
        let key = tile_order(position);
        if last.is_some_and(|last| last >= key) {
            return malformed();
        }
        last = Some(key);
    }
    Ok(())
}

fn check_snapshot(snapshot: &WorldMapViewSnapshot) -> Result<()> {
    check_header(&snapshot.header)?;
    if snapshot.tiles.len() > MAX_SNAPSHOT_TILES {
        return Err(WorldMapWireError::LimitExceeded);
    }
    let origin = snapshot.header.origin;
    snapshot
        .tiles
        .iter()
        .try_for_each(|tile| check_tile(origin, tile))?;
    check_ascending(snapshot.tiles.iter().map(|tile| &tile.position))
}

/// The view a delta applies to: the header of the last snapshot, with the origin and
/// `first_visible_floor` of the last header applied.
pub type MapViewBinding = MapViewHeader;

fn check_delta(view: &MapViewBinding, delta: &WorldMapViewDelta) -> Result<()> {
    check_header(&delta.header)?;
    if delta.tiles.len() + delta.cleared.len() > MAX_DELTA_ENTRIES {
        return Err(WorldMapWireError::LimitExceeded);
    }
    let (from, to) = (view.origin, delta.header.origin);
    let step = i64::from(to.x)
        .abs_diff(i64::from(from.x))
        .max(i64::from(to.y).abs_diff(i64::from(from.y)));
    if from.floor != to.floor || step > 1 {
        return malformed();
    }
    if delta.tiles.is_empty()
        && delta.cleared.is_empty()
        && from == to
        && delta.header.first_visible_floor == view.first_visible_floor
    {
        return malformed();
    }
    delta
        .tiles
        .iter()
        .try_for_each(|tile| check_tile(to, tile))?;
    if delta
        .cleared
        .iter()
        .any(|position| !in_window(to, *position))
    {
        return malformed();
    }
    check_ascending(delta.tiles.iter().map(|tile| &tile.position))?;
    check_ascending(delta.cleared.iter())?;
    let mut tiles = delta.tiles.iter().map(|tile| tile_order(&tile.position));
    let mut next = tiles.next();
    for cleared in delta.cleared.iter().map(tile_order) {
        while next.is_some_and(|tile| tile < cleared) {
            next = tiles.next();
        }
        if next == Some(cleared) {
            return malformed();
        }
    }
    Ok(())
}

// --- encoding --------------------------------------------------------------------------------

const fn varint_len(value: u64) -> usize {
    let bits = 64 - (value | 1).leading_zeros() as usize;
    bits.div_ceil(7)
}

const fn zigzag(value: i32) -> u64 {
    ((value << 1) ^ (value >> 31)) as u32 as u64
}

fn field_len(value: u64) -> usize {
    if value == 0 { 0 } else { 1 + varint_len(value) }
}

fn push_field(output: &mut Vec<u8>, field: u8, value: u64) {
    if value != 0 {
        output.push(field << 3);
        push_varint(output, value);
    }
}

fn push_len(output: &mut Vec<u8>, field: u8, len: usize) {
    output.push((field << 3) | 2);
    push_varint(output, len as u64);
}

fn position_len(position: &ActorPosition) -> usize {
    field_len(zigzag(position.x))
        + field_len(zigzag(position.y))
        + field_len(zigzag(i32::from(position.floor)))
}

fn push_position(output: &mut Vec<u8>, field: u8, position: &ActorPosition) {
    push_len(output, field, position_len(position));
    push_field(output, 1, zigzag(position.x));
    push_field(output, 2, zigzag(position.y));
    push_field(output, 3, zigzag(i32::from(position.floor)));
}

const fn framed(len: usize) -> usize {
    1 + varint_len(len as u64) + len
}

fn item_len(item: &MapItem) -> usize {
    let reference = match item.definition {
        MapDefinition::Item(reference) | MapDefinition::Terrain(reference) => reference.get(),
    };
    let origin = match item.origin {
        MapOrigin::BaseOrdinal {
            ordinal,
            object_revision,
        } => 1 + varint_len(u64::from(ordinal)) + field_len(object_revision),
        MapOrigin::Handle(handle) => 1 + varint_len(handle.get()),
        MapOrigin::DisplayOnly => 2,
    };
    1 + varint_len(u64::from(reference))
        + field_len(u64::from(item.count))
        + field_len(u64::from(item.sub_type))
        + origin
        + field_len(u64::from(item.appearance_id))
}

fn push_item(output: &mut Vec<u8>, item: &MapItem) {
    push_len(output, 2, item_len(item));
    match item.definition {
        MapDefinition::Item(reference) => push_field(output, 1, u64::from(reference.get())),
        MapDefinition::Terrain(_) => {}
    }
    push_field(output, 2, u64::from(item.count));
    push_field(output, 3, u64::from(item.sub_type));
    match item.origin {
        MapOrigin::BaseOrdinal { ordinal, .. } => {
            // A zero ordinal is still the set oneof member, so it is always emitted.
            output.push(4 << 3);
            push_varint(output, u64::from(ordinal));
        }
        MapOrigin::Handle(handle) => push_field(output, 5, handle.get()),
        MapOrigin::DisplayOnly => push_field(output, 6, 1),
    }
    push_field(output, 7, u64::from(item.appearance_id));
    if let MapDefinition::Terrain(reference) = item.definition {
        push_field(output, 8, u64::from(reference.get()));
    }
    if let MapOrigin::BaseOrdinal {
        object_revision, ..
    } = item.origin
    {
        push_field(output, 9, object_revision);
    }
}

fn tile_len(tile: &MapTile) -> usize {
    framed(position_len(&tile.position))
        + tile
            .items
            .iter()
            .map(|item| framed(item_len(item)))
            .sum::<usize>()
        + usize::from(tile.more) * 2
        + field_len(u64::from(tile.ground_speed))
}

fn push_tile(output: &mut Vec<u8>, field: u8, tile: &MapTile) {
    push_len(output, field, tile_len(tile));
    push_position(output, 1, &tile.position);
    for item in &tile.items {
        push_item(output, item);
    }
    push_field(output, 3, u64::from(tile.more));
    push_field(output, 4, u64::from(tile.ground_speed));
}

fn header_len(header: &MapViewHeader) -> usize {
    2 * framed(32)
        + field_len(header.reset_epoch)
        + framed(position_len(&header.origin))
        + field_len(zigzag(i32::from(header.first_visible_floor)))
}

fn push_header(output: &mut Vec<u8>, header: &MapViewHeader) {
    push_len(output, 1, header_len(header));
    push_len(output, 1, 32);
    output.extend_from_slice(&header.content_generation);
    push_len(output, 2, 32);
    output.extend_from_slice(&header.bundle_digest);
    push_field(output, 3, header.reset_epoch);
    push_position(output, 4, &header.origin);
    push_field(output, 5, zigzag(i32::from(header.first_visible_floor)));
}

fn check_tile_bytes(tile: &MapTile) -> Result<()> {
    if tile
        .items
        .iter()
        .any(|item| item_len(item) > MAX_MAP_ITEM_BYTES)
        || framed(tile_len(tile)) > MAX_MAP_TILE_BYTES
    {
        return Err(WorldMapWireError::LimitExceeded);
    }
    Ok(())
}

fn encoded_len(header: &MapViewHeader, tiles: &[MapTile], cleared: &[ActorPosition]) -> usize {
    framed(header_len(header))
        + tiles
            .iter()
            .map(|tile| framed(tile_len(tile)))
            .sum::<usize>()
        + cleared
            .iter()
            .map(|position| framed(position_len(position)))
            .sum::<usize>()
}

/// Encodes a domain-17 snapshot payload; refuses what the decoder refuses.
pub fn encode_world_map_snapshot(snapshot: &WorldMapViewSnapshot) -> Result<Vec<u8>> {
    check_snapshot(snapshot)?;
    snapshot.tiles.iter().try_for_each(check_tile_bytes)?;
    let len = encoded_len(&snapshot.header, &snapshot.tiles, &[]);
    if len > MAX_SNAPSHOT_PAYLOAD_BYTES {
        return Err(WorldMapWireError::LimitExceeded);
    }
    let mut output = Vec::with_capacity(len);
    push_header(&mut output, &snapshot.header);
    for tile in &snapshot.tiles {
        push_tile(&mut output, 2, tile);
    }
    debug_assert_eq!(output.len(), len);
    Ok(output)
}

/// Encodes a domain-17 delta payload against the view it applies to; refuses what the decoder
/// refuses.
pub fn encode_world_map_delta(view: &MapViewBinding, delta: &WorldMapViewDelta) -> Result<Vec<u8>> {
    if !view.same_binding(&delta.header) {
        return Err(WorldMapWireError::BindingMismatch);
    }
    check_delta(view, delta)?;
    delta.tiles.iter().try_for_each(check_tile_bytes)?;
    let len = encoded_len(&delta.header, &delta.tiles, &delta.cleared);
    if len > MAX_DELTA_PAYLOAD_BYTES {
        return Err(WorldMapWireError::LimitExceeded);
    }
    let mut output = Vec::with_capacity(len);
    push_header(&mut output, &delta.header);
    for tile in &delta.tiles {
        push_tile(&mut output, 2, tile);
    }
    for position in &delta.cleared {
        push_position(&mut output, 3, position);
    }
    debug_assert_eq!(output.len(), len);
    Ok(output)
}

// --- decoding --------------------------------------------------------------------------------

fn wire(result: crate::charm_wire::WireResult<u64>) -> Result<u64> {
    result.map_err(|_| WorldMapWireError::Malformed)
}

fn varint(input: &[u8], cursor: &mut usize) -> Result<u64> {
    wire(read_varint(input, cursor))
}

fn bytes<'a>(input: &'a [u8], cursor: &mut usize) -> Result<&'a [u8]> {
    read_bytes(input, cursor).map_err(|_| WorldMapWireError::Malformed)
}

fn once<T>(slot: &mut Option<T>, value: T) -> Result<()> {
    if slot.is_some() {
        return malformed();
    }
    *slot = Some(value);
    Ok(())
}

fn uint32(value: u64) -> Result<u32> {
    u32::try_from(value).map_err(|_| WorldMapWireError::Malformed)
}

fn sint32(value: u64) -> Result<i32> {
    let zigzag = uint32(value)?;
    Ok(((zigzag >> 1) as i32) ^ -((zigzag & 1) as i32))
}

fn decode_position(input: &[u8]) -> Result<ActorPosition> {
    let mut cursor = 0;
    let (mut x, mut y, mut floor) = (None, None, None);
    while cursor < input.len() {
        let slot = match varint(input, &mut cursor)? {
            0x08 => &mut x,
            0x10 => &mut y,
            0x18 => &mut floor,
            _ => return malformed(),
        };
        once(slot, sint32(varint(input, &mut cursor)?)?)?;
    }
    Ok(ActorPosition {
        x: x.unwrap_or(0),
        y: y.unwrap_or(0),
        floor: i16::try_from(floor.unwrap_or(0)).map_err(|_| WorldMapWireError::Malformed)?,
    })
}

fn decode_header(input: &[u8]) -> Result<MapViewHeader> {
    let mut cursor = 0;
    let (mut generation, mut digest, mut epoch, mut origin, mut visible) =
        (None, None, None, None, None);
    while cursor < input.len() {
        match varint(input, &mut cursor)? {
            0x0a => once(
                &mut generation,
                <[u8; 32]>::try_from(bytes(input, &mut cursor)?)
                    .map_err(|_| WorldMapWireError::Malformed)?,
            )?,
            0x12 => once(
                &mut digest,
                <[u8; 32]>::try_from(bytes(input, &mut cursor)?)
                    .map_err(|_| WorldMapWireError::Malformed)?,
            )?,
            0x18 => once(&mut epoch, varint(input, &mut cursor)?)?,
            0x22 => once(&mut origin, decode_position(bytes(input, &mut cursor)?)?)?,
            0x28 => once(&mut visible, sint32(varint(input, &mut cursor)?)?)?,
            _ => return malformed(),
        }
    }
    let (Some(content_generation), Some(bundle_digest), Some(origin)) =
        (generation, digest, origin)
    else {
        return malformed();
    };
    Ok(MapViewHeader {
        content_generation,
        bundle_digest,
        reset_epoch: epoch.unwrap_or(0),
        origin,
        first_visible_floor: i16::try_from(visible.unwrap_or(0))
            .map_err(|_| WorldMapWireError::Malformed)?,
    })
}

fn decode_item(input: &[u8]) -> Result<MapItem> {
    let mut cursor = 0;
    let (mut item_ref, mut terrain_ref, mut count, mut sub_type) = (None, None, None, None);
    let (mut ordinal, mut handle, mut display, mut appearance, mut revision) =
        (None, None, None, None, None);
    while cursor < input.len() {
        let key = varint(input, &mut cursor)?;
        let value = match key {
            0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 | 0x38 | 0x40 | 0x48 => {
                varint(input, &mut cursor)?
            }
            _ => return malformed(),
        };
        match key >> 3 {
            1 => once(&mut item_ref, uint32(value)?)?,
            2 => once(&mut count, uint32(value)?)?,
            3 => once(&mut sub_type, uint32(value)?)?,
            4 => once(&mut ordinal, value)?,
            5 => once(&mut handle, value)?,
            6 => once(&mut display, value)?,
            7 => once(&mut appearance, value)?,
            8 => once(&mut terrain_ref, uint32(value)?)?,
            _ => once(&mut revision, value)?,
        }
    }
    let definition = match (item_ref, terrain_ref) {
        (Some(reference), None) => {
            MapDefinition::Item(NonZeroU32::new(reference).ok_or(WorldMapWireError::Malformed)?)
        }
        (None, Some(reference)) => {
            MapDefinition::Terrain(NonZeroU32::new(reference).ok_or(WorldMapWireError::Malformed)?)
        }
        _ => return malformed(),
    };
    let origin = match (ordinal, handle, display) {
        (Some(ordinal), None, None) => MapOrigin::BaseOrdinal {
            ordinal: u8::try_from(ordinal)
                .ok()
                .filter(|ordinal| *ordinal <= MAX_BASE_ORDINAL)
                .ok_or(WorldMapWireError::Malformed)?,
            object_revision: revision.unwrap_or(0),
        },
        (None, Some(handle), None) if revision.is_none() => {
            MapOrigin::Handle(NonZeroU64::new(handle).ok_or(WorldMapWireError::Malformed)?)
        }
        (None, None, Some(1)) if revision.is_none() => MapOrigin::DisplayOnly,
        _ => return malformed(),
    };
    let appearance_id = u16::try_from(appearance.unwrap_or(0))
        .ok()
        .filter(|id| u32::from(*id) <= MAX_APPEARANCE_ID)
        .ok_or(WorldMapWireError::Malformed)?;
    let item = MapItem {
        definition,
        count: count.unwrap_or(0),
        sub_type: sub_type.unwrap_or(0),
        origin,
        appearance_id,
    };
    check_item(&item)?;
    Ok(item)
}

fn decode_tile(input: &[u8]) -> Result<MapTile> {
    if framed(input.len()) > MAX_MAP_TILE_BYTES {
        return Err(WorldMapWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut position, mut more, mut speed) = (None, None, None);
    let mut items = Vec::new();
    while cursor < input.len() {
        match varint(input, &mut cursor)? {
            0x0a => once(&mut position, decode_position(bytes(input, &mut cursor)?)?)?,
            0x12 => {
                if items.len() == MAX_TILE_ITEMS {
                    return Err(WorldMapWireError::LimitExceeded);
                }
                let item = bytes(input, &mut cursor)?;
                if item.len() > MAX_MAP_ITEM_BYTES {
                    return Err(WorldMapWireError::LimitExceeded);
                }
                items.push(decode_item(item)?);
            }
            0x18 => once(&mut more, varint(input, &mut cursor)?)?,
            0x20 => once(&mut speed, varint(input, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    let more = match more {
        None => false,
        Some(1) => true,
        Some(_) => return malformed(),
    };
    Ok(MapTile {
        position: position.ok_or(WorldMapWireError::Malformed)?,
        items,
        more,
        ground_speed: u16::try_from(speed.unwrap_or(0))
            .ok()
            .filter(|speed| *speed <= MAX_GROUND_SPEED)
            .ok_or(WorldMapWireError::Malformed)?,
    })
}

/// Decodes a domain-17 snapshot payload.
pub fn decode_world_map_snapshot(payload: &[u8]) -> Result<WorldMapViewSnapshot> {
    if payload.len() > MAX_SNAPSHOT_PAYLOAD_BYTES {
        return Err(WorldMapWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut header = None;
    let mut tiles = Vec::new();
    while cursor < payload.len() {
        match varint(payload, &mut cursor)? {
            0x0a => once(&mut header, decode_header(bytes(payload, &mut cursor)?)?)?,
            0x12 => {
                if tiles.len() == MAX_SNAPSHOT_TILES {
                    return Err(WorldMapWireError::LimitExceeded);
                }
                tiles.push(decode_tile(bytes(payload, &mut cursor)?)?);
            }
            _ => return malformed(),
        }
    }
    let snapshot = WorldMapViewSnapshot {
        header: header.ok_or(WorldMapWireError::Malformed)?,
        tiles,
    };
    check_snapshot(&snapshot)?;
    Ok(snapshot)
}

/// Decodes a domain-17 delta payload against the view it applies to. Another binding is
/// [`WorldMapWireError::BindingMismatch`].
pub fn decode_world_map_delta(view: &MapViewBinding, payload: &[u8]) -> Result<WorldMapViewDelta> {
    if payload.len() > MAX_DELTA_PAYLOAD_BYTES {
        return Err(WorldMapWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut header = None;
    let (mut tiles, mut cleared) = (Vec::new(), Vec::new());
    while cursor < payload.len() {
        match varint(payload, &mut cursor)? {
            0x0a => once(&mut header, decode_header(bytes(payload, &mut cursor)?)?)?,
            key @ (0x12 | 0x1a) => {
                if tiles.len() + cleared.len() == MAX_DELTA_ENTRIES {
                    return Err(WorldMapWireError::LimitExceeded);
                }
                let field = bytes(payload, &mut cursor)?;
                if key == 0x12 {
                    tiles.push(decode_tile(field)?);
                } else {
                    cleared.push(decode_position(field)?);
                }
            }
            _ => return malformed(),
        }
    }
    let header = header.ok_or(WorldMapWireError::Malformed)?;
    if !view.same_binding(&header) {
        return Err(WorldMapWireError::BindingMismatch);
    }
    let delta = WorldMapViewDelta {
        header,
        tiles,
        cleared,
    };
    check_delta(view, &delta)?;
    Ok(delta)
}

#[cfg(test)]
#[path = "world_map_tests.rs"]
mod tests;
