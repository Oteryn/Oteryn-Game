//! MAP-WIRE-2: the composed tile stacks of a World map view (MAP-WIRE-1 contract §2, §3).
//!
//! - **Composition.** A tile's composed stack is its base top-level entries in payload order, minus
//!   the ones the Channel overlay hides, plus the overlay's added and Ground items in insertion
//!   order. A base entry's contents (depth above 0) are never on the map.
//! - **Facts.** [`WorldBase`] keeps only compact ids, and Item and Terrain compact ids are
//!   separate spaces, so every per-entry fact (family-tagged definition reference, Terrain kind,
//!   appearance, `blocks_projectile`, pickup eligibility, binding, overlay revision) is injected
//!   through [`MapFacts`] by the owner that holds the bundle palette and the content generation.
//!   A missing fact fails closed.
//! - **Cut.** The wire carries the bottom entry and the 9 topmost entries, with `more` when the
//!   stack is longer ([`cut`]). Visible floors read the full composed stack, never the cut.
//! - **Window.** 18x14 tiles from `(x - 8, y - 6)` over every floor in view, each floor shifted by
//!   its difference to the actor's floor ([`window`]).
//! - **Visible floors.** [`first_visible_floor`] is the contract §2 rule as a pure function of the
//!   actor position and the composed stacks.
//! - **Handle budget.** [`budget_order`] ranks the handle-bearing entries in view: the actor's
//!   floor first, then floor distance, Chebyshev distance to the actor's perspective position,
//!   `(y, x)` and stack order. The first `MAPW-RL-04` carry handles.
//!
//! No runtime holds a [`WorldBase`] until MAP-CUTOVER-1, so nothing here is reached in
//! production yet.

#![cfg_attr(not(test), allow(dead_code))]

use super::WorldBase;
use super::overlay::{AddedItem, ChannelOverlay, EntryId, TilePos};
use oteryn_protocol_oteryn::world_map::{
    MAX_BASE_ORDINAL, MAX_TILE_ITEMS, MapDefinition, VIEW_HEIGHT, VIEW_LEFT, VIEW_TOP, VIEW_WIDTH,
    first_visible_floor_start, floors_in_view,
};
use oteryn_world_bundle::bundle::TerrainKind;

/// The facts of one composed entry that the view needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryFacts {
    /// `item_definition_ref` for an Item palette entry and every added or Ground item,
    /// `terrain_definition_ref` (the palette compact id) for a Terrain palette entry.
    pub definition: MapDefinition,
    /// The Terrain kind of the entry's record, `None` for a plain Item.
    pub terrain_kind: Option<TerrainKind>,
    /// The 15.30 appearance object id ([`appearance_id`]), 0 if none.
    pub appearance_id: u16,
    /// The entry's item definition has `blocks_projectile`.
    pub blocks_projectile: bool,
    /// The entry's item definition is pickupable (ADR-0021 §4.4).
    pub pickupable: bool,
    /// The placement has an action, unique, door, depot or teleport binding, or an attribute the
    /// pickup cannot represent (ADR-0021 §4.4).
    pub bound: bool,
    /// `1..=100`.
    pub count: u32,
    pub sub_type: u32,
}

/// The bundle and content facts of a map view, held by the owner of the bundle palette and the
/// content generation.
pub trait MapFacts {
    /// The facts of top-level base entry `ordinal` of `pos`, whose compact id is `id`.
    fn base_entry(&self, pos: TilePos, ordinal: u8, id: u32) -> Option<EntryFacts>;
    /// The facts of an overlay-added or Ground item, from its item content definition.
    fn added_entry(&self, item: &AddedItem) -> Option<EntryFacts>;
    /// Whether `pos` is a house tile.
    fn house_tile(&self, pos: TilePos) -> bool;
    /// The overlay revision of a base placement key in the current reset epoch, 0 while the
    /// overlay holds no state for it (contract §3 Object revision).
    fn object_revision(&self, placement_key: u64) -> u64;
}

/// Where a composed entry comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntrySource {
    /// An unhidden base entry. `movable` entries carry a handle (contract §3 Move source).
    Base {
        ordinal: u8,
        placement_key: u64,
        movable: bool,
    },
    /// An overlay-added volatile item.
    Added(EntryId),
    /// A durable Ground item.
    Ground([u8; 16]),
}

impl EntrySource {
    /// Whether the entry carries a handle within the budget.
    pub const fn handle_bearing(&self) -> bool {
        match self {
            Self::Base { movable, .. } => *movable,
            Self::Added(_) | Self::Ground(_) => true,
        }
    }
}

/// The identity a handle-bearing entry's handle is bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MapEntryKey {
    /// A movable base entry, by its bundle placement key.
    Base(u64),
    Added(EntryId),
    Ground([u8; 16]),
}

impl EntrySource {
    pub const fn key(&self) -> MapEntryKey {
        match self {
            Self::Base { placement_key, .. } => MapEntryKey::Base(*placement_key),
            Self::Added(id) => MapEntryKey::Added(*id),
            Self::Ground(id) => MapEntryKey::Ground(*id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposedEntry {
    pub source: EntrySource,
    pub facts: EntryFacts,
}

/// The full composed stack of one tile, bottom first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedTile {
    pub pos: TilePos,
    pub entries: Vec<ComposedEntry>,
    /// The base ground item's speed (0 without one).
    pub ground_speed: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewError {
    /// A fact the view needs is missing.
    MissingFacts,
    /// A base entry beyond the 64-entry ordinal reach is unhidden.
    OrdinalReach,
}

/// The bundle placement key of a top-level base entry (format §7):
/// `x << 32 | y << 16 | (-floor) << 8 | ordinal`.
pub fn placement_key(pos: TilePos, ordinal: u8) -> Option<u64> {
    let depth = u8::try_from(-i16::from(pos.floor)).ok()?;
    Some(
        u64::from(pos.x) << 32
            | u64::from(pos.y) << 16
            | u64::from(depth) << 8
            | u64::from(ordinal),
    )
}

/// The tile and ordinal of a bundle placement key; `None` for a key no top-level entry can have.
pub fn placement_of(key: u64) -> Option<(TilePos, u8)> {
    if key >> 48 != 0 {
        return None;
    }
    let depth = (key >> 8 & 0xFF) as u8;
    let ordinal = (key & 0xFF) as u8;
    if depth > 15 || ordinal > MAX_BASE_ORDINAL {
        return None;
    }
    let pos = TilePos {
        x: (key >> 32 & 0xFFFF) as u16,
        y: (key >> 16 & 0xFFFF) as u16,
        floor: -(depth as i8),
    };
    Some((pos, ordinal))
}

/// The appearance id of a definition key (contract §3 Appearance): `<id>` of an
/// `oteryn:item.tibia.i<id>` or `oteryn:terrain.tibia.i<id>` key, the `source_item_id` of a
/// provisional donor key, otherwise 0. An id above 65,535 is no appearance.
pub fn appearance_id(key: &str, donor_source_item_id: Option<u32>) -> u16 {
    let tibia = ["oteryn:item.tibia.i", "oteryn:terrain.tibia.i"]
        .iter()
        .find_map(|prefix| key.strip_prefix(prefix));
    let id = match tibia {
        Some(digits)
            if !digits.is_empty()
                && digits.bytes().all(|byte| byte.is_ascii_digit())
                && !(digits.len() > 1 && digits.starts_with('0')) =>
        {
            digits.parse::<u32>().ok()
        }
        Some(_) => None,
        None => donor_source_item_id,
    };
    id.and_then(|id| u16::try_from(id).ok()).unwrap_or(0)
}

/// The composed stack of `pos` (contract §2): `None` when the tile is empty.
pub fn compose_tile(
    overlay: &ChannelOverlay,
    facts: &impl MapFacts,
    pos: TilePos,
) -> Result<Option<ComposedTile>, ViewError> {
    let base: &WorldBase = overlay.base();
    let view = overlay.tile(pos);
    let mut entries = Vec::new();
    let mut ground_speed = 0;
    if let Some(tile) = base.tile(pos.x, pos.y, pos.floor) {
        ground_speed = tile.stored_ground_speed();
        let (ids, depths) = (tile.ids(), tile.depths());
        let mut ordinal: u16 = 0;
        for (at, (id, depth)) in ids.iter().zip(depths).enumerate() {
            if *depth != 0 {
                continue;
            }
            let this = ordinal;
            ordinal += 1;
            let hidden = u8::try_from(this)
                .ok()
                .is_some_and(|ordinal| view.is_some_and(|view| view.is_hidden(ordinal)));
            if hidden {
                continue;
            }
            let ordinal = u8::try_from(this)
                .ok()
                .filter(|ordinal| *ordinal <= MAX_BASE_ORDINAL)
                .ok_or(ViewError::OrdinalReach)?;
            let entry = facts
                .base_entry(pos, ordinal, *id)
                .ok_or(ViewError::MissingFacts)?;
            let contents = depths.get(at + 1).is_some_and(|next| *next != 0);
            let movable = entry.pickupable && !entry.bound && !contents && !facts.house_tile(pos);
            entries.push(ComposedEntry {
                source: EntrySource::Base {
                    ordinal,
                    placement_key: placement_key(pos, ordinal).ok_or(ViewError::OrdinalReach)?,
                    movable,
                },
                facts: entry,
            });
        }
    }
    if let Some(view) = view {
        for added in view.added() {
            let entry = facts
                .added_entry(added.item())
                .ok_or(ViewError::MissingFacts)?;
            let source = match added.item() {
                AddedItem::Volatile { .. } => EntrySource::Added(added.id()),
                AddedItem::Ground(ground) => EntrySource::Ground(ground.item_instance_id),
            };
            entries.push(ComposedEntry {
                source,
                facts: entry,
            });
        }
    }
    if entries.is_empty() {
        return Ok(None);
    }
    Ok(Some(ComposedTile {
        pos,
        entries,
        ground_speed,
    }))
}

/// The cut of a composed stack (contract §2): the bottom entry and the 9 topmost, in stack
/// order, and whether entries were dropped.
pub fn cut<T>(entries: &[T]) -> (Vec<&T>, bool) {
    if entries.len() <= MAX_TILE_ITEMS {
        return (entries.iter().collect(), false);
    }
    let top = entries.len() - (MAX_TILE_ITEMS - 1);
    let kept = std::iter::once(&entries[0])
        .chain(&entries[top..])
        .collect();
    (kept, true)
}

/// The positions of the window of `origin` (contract §2 Viewport): every floor in view, each
/// shifted by its floor difference to the origin, clipped to the native plane.
pub fn window(origin: TilePos) -> Vec<TilePos> {
    let Some(floors) = floors_in_view(i16::from(origin.floor)) else {
        return Vec::new();
    };
    let mut positions = Vec::new();
    for floor in floors {
        let shift = i32::from(floor) - i32::from(origin.floor);
        let left = i32::from(origin.x) - VIEW_LEFT + shift;
        let top = i32::from(origin.y) - VIEW_TOP + shift;
        for y in top..top + VIEW_HEIGHT {
            for x in left..left + VIEW_WIDTH {
                if let (Ok(x), Ok(y), Ok(floor)) =
                    (u16::try_from(x), u16::try_from(y), i8::try_from(floor))
                {
                    positions.push(TilePos { x, y, floor });
                }
            }
        }
    }
    positions
}

/// What the visible-floor rule reads of one composed stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RoofFacts {
    /// The bottom entry is a Terrain entry of kind `ground`, `wall` or `roof`.
    pub limits_view: bool,
    /// No entry is a `wall` and no entry's item definition has `blocks_projectile`.
    pub look_through: bool,
}

impl RoofFacts {
    /// The facts of a full composed stack; an empty tile can be looked through.
    pub fn of(tile: Option<&ComposedTile>) -> Self {
        let Some(tile) = tile else {
            return Self {
                limits_view: false,
                look_through: true,
            };
        };
        let limits_view = tile.entries.first().is_some_and(|bottom| {
            matches!(
                bottom.facts.terrain_kind,
                Some(TerrainKind::Ground | TerrainKind::Wall | TerrainKind::Roof)
            )
        });
        let look_through = tile.entries.iter().all(|entry| {
            entry.facts.terrain_kind != Some(TerrainKind::Wall) && !entry.facts.blocks_projectile
        });
        Self {
            limits_view,
            look_through,
        }
    }
}

/// The positions the visible-floor rule reads for an actor at `actor`: the actor's tile and its
/// orthogonal neighbours, each with up to 7 floors above, at the tile above and the tile that
/// covers it in perspective. At most 5 x 7 x 2 = 70 tiles.
pub fn roof_positions(actor: TilePos) -> Vec<TilePos> {
    let mut positions = Vec::new();
    for (dx, dy) in [(0, 0), (0, -1), (1, 0), (0, 1), (-1, 0)] {
        positions.extend(offset(actor, dx, dy, 0));
        for k in 1..=7 {
            positions.extend(offset(actor, dx, dy, k));
            positions.extend(offset(actor, dx + k, dy + k, k));
        }
    }
    positions.sort_unstable();
    positions.dedup();
    positions
}

fn offset(pos: TilePos, dx: i32, dy: i32, up: i32) -> Option<TilePos> {
    let floor = i8::try_from(i32::from(pos.floor) + up).ok()?;
    if floor > 0 {
        return None;
    }
    Some(TilePos {
        x: u16::try_from(i32::from(pos.x) + dx).ok()?,
        y: u16::try_from(i32::from(pos.y) + dy).ok()?,
        floor,
    })
}

/// `first_visible_floor` for an actor at `actor` (contract §2 Visible floors), reading each
/// position's full composed stack through `roof`.
pub fn first_visible_floor(actor: TilePos, roof: impl Fn(TilePos) -> RoofFacts) -> i8 {
    let f = i32::from(actor.floor);
    let mut first = i32::from(first_visible_floor_start(i16::from(actor.floor)));
    for (index, (dx, dy)) in [(0, 0), (0, -1), (1, 0), (0, 1), (-1, 0)]
        .into_iter()
        .enumerate()
    {
        let Some(at) = offset(actor, dx, dy, 0) else {
            continue;
        };
        if index != 0 && !roof(at).look_through {
            continue;
        }
        let mut k = 1;
        while f + k <= first && first > f {
            let above = offset(at, 0, 0, k).map(&roof);
            let covering = offset(at, k, k, k).map(&roof);
            if above.is_some_and(|facts| facts.limits_view)
                || covering.is_some_and(|facts| facts.limits_view)
            {
                first = f + k - 1;
                break;
            }
            k += 1;
        }
    }
    i8::try_from(first).unwrap_or(actor.floor)
}

/// The ranking key of a handle-bearing entry (contract §3 Handle budget): the actor's floor
/// first, then floor distance; within a floor, Chebyshev distance to the actor's position in that
/// floor's perspective, then `(y, x)`, then stack order.
pub fn budget_order(actor: TilePos, pos: TilePos, stack_index: usize) -> impl Ord {
    let shift = i32::from(pos.floor) - i32::from(actor.floor);
    let (cx, cy) = (i32::from(actor.x) + shift, i32::from(actor.y) + shift);
    let chebyshev = (i32::from(pos.x) - cx)
        .abs()
        .max((i32::from(pos.y) - cy).abs());
    (
        shift != 0,
        shift.abs(),
        chebyshev,
        pos.y,
        pos.x,
        stack_index,
    )
}
