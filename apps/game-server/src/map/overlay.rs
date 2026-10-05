//! MAP-OVERLAY-1a: the per-channel overlay over a World's shared base map (ADR-0021 §4.4, §4.8).
//!
//! Every Channel of a World shares one [`WorldBase`] by `Arc`; a [`ChannelOverlay`] holds only
//! what that Channel changed:
//! - per tile, the base top-level entries it hides (a `u64` bitmask over top-level ordinals, the
//!   reach of the 64-entry tile limit) and the items it added, each with its full attributes;
//! - an expiry index that removes a decayed volatile item at 1 s granularity;
//! - the `MAP01-CHANNEL-OVERLAY-BYTES` budget.
//!
//! An added item is never merged with a base entry or another added item, so each base
//! `placement_key` keeps exactly one entry. A new volatile entry or a freeze-time hide that would
//! exceed the budget is refused before anything is allocated; a durable Ground item and a rebuild
//! re-hide are always admitted, counted, and raise the alarm on overflow.
//!
//! After a restart [`ChannelOverlay::rebuild`] rebuilds every durable Ground item of the channel
//! and fails closed if any item's `map_revision` is not the active bundle digest. There are no
//! overlay snapshots and no overlay journal.

use super::WorldBase;
use crate::durability::item_mint::GroundItemInstance;
use crate::durability::spell_items_abi::decode_ground_cell;
use crate::foundation::{ChannelId, WorldId};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::mem::size_of;
use std::sync::Arc;

/// `MAP01-CHANNEL-OVERLAY-BYTES`: the default budget of one channel overlay.
pub const OVERLAY_BUDGET_BYTES: usize = 64 * 1024 * 1024;

/// The reach of the hidden-item bitmask: top-level ordinals `0..64`.
pub const HIDDEN_REACH: u8 = 64;

/// A native tile position: `floor` is `-15..=0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TilePos {
    pub x: u16,
    pub y: u16,
    pub floor: i8,
}

/// The overlay-local identity of one added entry; never reused within an overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId(u64);

impl EntryId {
    pub fn get(self) -> u64 {
        self.0
    }
}

/// A volatile added item: a moved or dropped map object. Never written to PostgreSQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolatileItem {
    /// The compact id, as in the base.
    pub id: u32,
    pub count: u16,
    /// The item's full attributes, kept as given.
    pub attributes: Vec<u8>,
}

/// An item added to a tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddedItem {
    /// Volatile overlay state; `decays_at_ms` (Unix ms) puts it in the expiry index.
    Volatile {
        item: VolatileItem,
        decays_at_ms: Option<u64>,
    },
    /// A durable DUR-03 Ground item, kept with its full record.
    Ground(Box<GroundItemInstance>),
}

/// One added entry of a tile, in insertion order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedEntry {
    id: EntryId,
    item: AddedItem,
    cost: usize,
}

impl AddedEntry {
    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn item(&self) -> &AddedItem {
        &self.item
    }
}

/// How a charge is admitted against the budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// Refused atomically if it would exceed the budget: a new volatile entry, a freeze-time hide.
    Refusable,
    /// Never refused: a durable Ground item, a rebuild re-hide. An overflow raises the alarm.
    Durable,
}

/// Why an overlay change was refused. A refused change leaves the overlay unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayError {
    /// The base has no tile at the position.
    NoBaseTile,
    /// The ordinal is not a top-level entry of the base tile.
    Ordinal,
    AlreadyHidden,
    NotHidden,
    /// The change would exceed the budget.
    OverBudget {
        needed: usize,
        available: usize,
    },
    /// The tile has no added entry with this id.
    NoEntry,
    /// The Ground item belongs to another World.
    World,
    /// The Ground item belongs to another Channel.
    Channel,
    /// The Ground item's `map_revision` is not the active bundle digest.
    MapRevision,
    /// The Ground item's position does not decode to a native tile position.
    Position,
    /// The Ground item is already in the overlay.
    DuplicateItem,
    /// The Ground item's persisted stack ordinal is zero or shared with another rebuilt item.
    StackOrdinal,
}

impl fmt::Display for OverlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoBaseTile => f.write_str("no base tile at the position"),
            Self::Ordinal => f.write_str("not a top-level ordinal of the base tile"),
            Self::AlreadyHidden => f.write_str("base entry already hidden"),
            Self::NotHidden => f.write_str("base entry not hidden"),
            Self::OverBudget { needed, available } => write!(
                f,
                "overlay budget exceeded: {needed} bytes needed, {available} available"
            ),
            Self::NoEntry => f.write_str("no added entry with this id on the tile"),
            Self::World => f.write_str("Ground item of another World"),
            Self::Channel => f.write_str("Ground item of another Channel"),
            Self::MapRevision => f.write_str("Ground item map_revision is not the active bundle"),
            Self::Position => f.write_str("Ground item position is not a native tile position"),
            Self::DuplicateItem => f.write_str("Ground item already in the overlay"),
            Self::StackOrdinal => f.write_str("Ground stack ordinal is zero or not unique"),
        }
    }
}

impl std::error::Error for OverlayError {}

/// One durable Ground item for a rebuild, with its persisted
/// `game_item_ground_locations.stack_ordinal`: the item owner stamps it from one ascending
/// sequence on insertion, so a higher ordinal lies above a lower one on the same tile and the
/// ordinals of one tile are unique but not contiguous.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundRebuildItem {
    pub stack_ordinal: u64,
    pub item: GroundItemInstance,
}

/// Why a Ground rebuild failed closed: the first item refused, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuildError {
    pub item_instance_id: [u8; 16],
    pub reason: OverlayError,
}

impl fmt::Display for RebuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ground rebuild failed closed: {}", self.reason)
    }
}

impl std::error::Error for RebuildError {}

#[derive(Debug, Default)]
struct TileOverlay {
    hidden: u64,
    added: Vec<AddedEntry>,
}

impl TileOverlay {
    fn is_empty(&self) -> bool {
        self.hidden == 0 && self.added.is_empty()
    }
}

/// A read view of one tile's overlay.
#[derive(Debug, Clone, Copy)]
pub struct TileOverlayView<'a> {
    tile: &'a TileOverlay,
}

impl<'a> TileOverlayView<'a> {
    /// The hidden top-level ordinals, bit `n` for ordinal `n`.
    pub fn hidden(&self) -> u64 {
        self.tile.hidden
    }

    pub fn is_hidden(&self, ordinal: u8) -> bool {
        ordinal < HIDDEN_REACH && self.tile.hidden & 1 << ordinal != 0
    }

    /// The added entries, in insertion order.
    pub fn added(&self) -> &'a [AddedEntry] {
        &self.tile.added
    }
}

type ExpiryKey = (u64, EntryId, TilePos);

// The accounted bytes are deterministic, conservative bounds of the heap each change can hold,
// so the gate needs no allocator query; docs/agents/evidence/MAP-OVERLAY-1a-overlay-budget.md
// measures them against the real heap.

/// One hash-table slot, held at most 3 per element (power-of-two buckets at 7/8 load, plus a
/// control byte each); the small-table remainder is in `FIXED_COST`.
const fn table_slot(element: usize) -> usize {
    3 * (element + 1)
}

/// The tables' minimum allocations and group padding.
const FIXED_COST: usize = 4096;
/// A tile record: its table slot plus the minimum 4-entry `Vec` of added entries.
const TILE_COST: usize = table_slot(size_of::<(TilePos, TileOverlay)>()) + 4 * ENTRY_SLOT;
/// An added entry's slot, doubled for `Vec` growth.
const ENTRY_SLOT: usize = size_of::<AddedEntry>();
/// A hidden origin: one bitmask bit, held as a byte so a hide is never free.
const HIDE_COST: usize = 8;
/// An expiry-index key, with B-tree nodes at least half full.
const EXPIRY_COST: usize = 4 * size_of::<ExpiryKey>();
/// A Ground item's index slot.
const GROUND_INDEX_COST: usize = table_slot(size_of::<([u8; 16], (TilePos, EntryId))>());

fn item_cost(item: &AddedItem) -> usize {
    2 * ENTRY_SLOT
        + match item {
            AddedItem::Volatile { item, decays_at_ms } => {
                item.attributes.capacity()
                    + if decays_at_ms.is_some() {
                        EXPIRY_COST
                    } else {
                        0
                    }
            }
            AddedItem::Ground(ground) => {
                size_of::<GroundItemInstance>()
                    + ground.definition.family.capacity()
                    + ground.definition.production_key.capacity()
                    + ground.definition.revision_ref.capacity()
                    + ground.ground.spatial_position.capacity()
                    + ground.ground.corpse_ref.capacity()
                    + ground.ground.map_revision.capacity()
                    + ground.ground.content_revision.capacity()
                    + ground.ground.native_room_placement_context.capacity()
                    + GROUND_INDEX_COST
            }
        }
}

/// The deadline second of a decay instant: the first whole second at or after it.
/// Whether a tile's `Vec` of added entries holds no more slots than were charged for it: the
/// minimum 4 in `TILE_COST` and 2 per entry in `item_cost`.
fn added_within_charge(added: &Vec<AddedEntry>) -> bool {
    added.capacity() <= 4 + 2 * added.len()
}

/// Gives back the slots a removal freed from a tile's added entries beyond their charges.
fn trim_added(added: &mut Vec<AddedEntry>) {
    if !added_within_charge(added) {
        added.shrink_to(added.len().max(4));
    }
}

/// Whether a hash table holds no more slots than its `table_slot` charges cover; the small-table
/// remainder of 16 slots is in `FIXED_COST`. Growth doubles, so only removal can break it.
fn table_within_charge<K, V>(table: &HashMap<K, V>) -> bool {
    table.capacity() <= 2 * table.len() + 16
}

/// Gives back what removals left a hash table holding beyond its charges.
fn trim_table<K: Eq + std::hash::Hash, V>(table: &mut HashMap<K, V>) {
    if !table_within_charge(table) {
        table.shrink_to(table.len());
    }
}

fn deadline_second(decays_at_ms: u64) -> u64 {
    decays_at_ms.div_ceil(1000)
}

/// The `map_revision` a Ground item placed on `base` carries.
pub fn map_revision(base: &WorldBase) -> String {
    let mut revision = String::with_capacity(71);
    revision.push_str("sha256:");
    for byte in base.digest() {
        revision.push_str(&format!("{byte:02x}"));
    }
    revision
}

/// One Channel's overlay over its World's shared base.
#[derive(Debug)]
pub struct ChannelOverlay {
    base: Arc<WorldBase>,
    world_id: WorldId,
    channel_id: ChannelId,
    map_revision: String,
    tiles: HashMap<TilePos, TileOverlay>,
    expiry: BTreeSet<ExpiryKey>,
    ground: HashMap<[u8; 16], (TilePos, EntryId)>,
    next_entry: u64,
    budget: usize,
    used: usize,
    alarms: u64,
}

impl ChannelOverlay {
    /// An empty overlay with the default budget.
    pub fn new(base: Arc<WorldBase>, world_id: WorldId, channel_id: ChannelId) -> Self {
        Self::with_budget(base, world_id, channel_id, OVERLAY_BUDGET_BYTES)
    }

    /// An empty overlay with `budget` bytes.
    pub fn with_budget(
        base: Arc<WorldBase>,
        world_id: WorldId,
        channel_id: ChannelId,
        budget: usize,
    ) -> Self {
        let map_revision = map_revision(&base);
        Self {
            base,
            world_id,
            channel_id,
            map_revision,
            tiles: HashMap::new(),
            expiry: BTreeSet::new(),
            ground: HashMap::new(),
            next_entry: 0,
            budget,
            used: FIXED_COST,
            alarms: 0,
        }
    }

    /// Rebuilds the channel's overlay after a restart from every durable Ground item it holds.
    /// The stack order comes from the persisted stack ordinals, never from the input order: the
    /// items are placed lowest ordinal first, so each tile's top item is its highest ordinal. A
    /// zero or repeated ordinal fails the rebuild closed before anything is placed. Every item
    /// is admitted whatever the budget; the first item that does not belong to this World,
    /// Channel and bundle, or does not decode to a base tile, fails the rebuild closed.
    pub fn rebuild(
        base: Arc<WorldBase>,
        world_id: WorldId,
        channel_id: ChannelId,
        budget: usize,
        items: impl IntoIterator<Item = GroundRebuildItem>,
    ) -> Result<Self, RebuildError> {
        let mut items: Vec<GroundRebuildItem> = items.into_iter().collect();
        items.sort_by_key(|entry| entry.stack_ordinal);
        let refused = items
            .first()
            .filter(|entry| entry.stack_ordinal == 0)
            .or_else(|| {
                items
                    .windows(2)
                    .find(|pair| pair[0].stack_ordinal == pair[1].stack_ordinal)
                    .map(|pair| &pair[1])
            });
        if let Some(entry) = refused {
            return Err(RebuildError {
                item_instance_id: entry.item.item_instance_id,
                reason: OverlayError::StackOrdinal,
            });
        }
        let mut overlay = Self::with_budget(base, world_id, channel_id, budget);
        for GroundRebuildItem { item, .. } in items {
            let item_instance_id = item.item_instance_id;
            overlay.add_ground(item).map_err(|reason| RebuildError {
                item_instance_id,
                reason,
            })?;
        }
        Ok(overlay)
    }

    pub fn base(&self) -> &Arc<WorldBase> {
        &self.base
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    pub fn budget(&self) -> usize {
        self.budget
    }

    /// The accounted bytes.
    pub fn used_bytes(&self) -> usize {
        self.used
    }

    /// Whether the accounted bytes exceed the budget (only durable admissions can do that).
    pub fn over_budget(&self) -> bool {
        self.used > self.budget
    }

    /// How many durable admissions landed over the budget: the operational alarm.
    pub fn alarm_count(&self) -> u64 {
        self.alarms
    }

    /// The overlay of the tile at `pos`; `None` if the channel changed nothing there.
    pub fn tile(&self, pos: TilePos) -> Option<TileOverlayView<'_>> {
        self.tiles.get(&pos).map(|tile| TileOverlayView { tile })
    }

    /// Every changed tile, in no particular order.
    pub fn tiles(&self) -> impl Iterator<Item = (TilePos, TileOverlayView<'_>)> + '_ {
        self.tiles
            .iter()
            .map(|(pos, tile)| (*pos, TileOverlayView { tile }))
    }

    /// Where a durable Ground item sits in this overlay.
    pub fn ground_entry(&self, item_instance_id: &[u8; 16]) -> Option<(TilePos, EntryId)> {
        self.ground.get(item_instance_id).copied()
    }

    /// Whether every retained structure holds no more capacity than the accounted bytes charge
    /// for, so `used_bytes` bounds the overlay's heap after any sequence of removals.
    pub fn capacity_within_charge(&self) -> bool {
        table_within_charge(&self.tiles)
            && table_within_charge(&self.ground)
            && self
                .tiles
                .values()
                .all(|tile| added_within_charge(&tile.added))
    }

    /// Admits `cost` bytes, or refuses without any change.
    fn admit(&mut self, cost: usize, admission: Admission) -> Result<(), OverlayError> {
        let after = self.used.saturating_add(cost);
        if after > self.budget {
            match admission {
                Admission::Refusable => {
                    return Err(OverlayError::OverBudget {
                        needed: cost,
                        available: self.budget.saturating_sub(self.used),
                    });
                }
                Admission::Durable => self.alarms += 1,
            }
        }
        self.used = after;
        Ok(())
    }

    fn tile_cost(&self, pos: TilePos) -> usize {
        if self.tiles.contains_key(&pos) {
            0
        } else {
            TILE_COST
        }
    }

    fn top_level_count(&self, pos: TilePos) -> Result<usize, OverlayError> {
        let tile = self
            .base
            .tile(pos.x, pos.y, pos.floor)
            .ok_or(OverlayError::NoBaseTile)?;
        Ok(tile.depths().iter().filter(|depth| **depth == 0).count())
    }

    /// Drops the tile record once the tile holds nothing, refunding it.
    fn release_tile(&mut self, pos: TilePos) {
        if self.tiles.get(&pos).is_some_and(TileOverlay::is_empty) {
            self.tiles.remove(&pos);
            self.used -= TILE_COST;
            trim_table(&mut self.tiles);
        }
    }

    /// Hides the base top-level entry `ordinal` of the tile at `pos`.
    pub fn hide(
        &mut self,
        pos: TilePos,
        ordinal: u8,
        admission: Admission,
    ) -> Result<(), OverlayError> {
        if ordinal >= HIDDEN_REACH || usize::from(ordinal) >= self.top_level_count(pos)? {
            return Err(OverlayError::Ordinal);
        }
        if self.tile(pos).is_some_and(|tile| tile.is_hidden(ordinal)) {
            return Err(OverlayError::AlreadyHidden);
        }
        self.admit(self.tile_cost(pos) + HIDE_COST, admission)?;
        self.tiles.entry(pos).or_default().hidden |= 1 << ordinal;
        Ok(())
    }

    /// Shows the hidden base top-level entry `ordinal` of the tile at `pos` again.
    pub fn unhide(&mut self, pos: TilePos, ordinal: u8) -> Result<(), OverlayError> {
        let tile = self
            .tiles
            .get_mut(&pos)
            .filter(|tile| ordinal < HIDDEN_REACH && tile.hidden & 1 << ordinal != 0)
            .ok_or(OverlayError::NotHidden)?;
        tile.hidden &= !(1 << ordinal);
        self.used -= HIDE_COST;
        self.release_tile(pos);
        Ok(())
    }

    /// Adds a volatile item on top of the tile at `pos`, refused atomically over the budget.
    pub fn add_volatile(
        &mut self,
        pos: TilePos,
        item: VolatileItem,
        decays_at_ms: Option<u64>,
    ) -> Result<EntryId, OverlayError> {
        self.top_level_count(pos)?;
        let item = AddedItem::Volatile { item, decays_at_ms };
        let cost = item_cost(&item);
        self.admit(self.tile_cost(pos) + cost, Admission::Refusable)?;
        let id = self.push(pos, item, cost);
        if let Some(decays_at_ms) = decays_at_ms {
            self.expiry.insert((deadline_second(decays_at_ms), id, pos));
        }
        Ok(id)
    }

    /// Adds a durable Ground item at its own position. Never refused for the budget.
    pub fn add_ground(&mut self, item: GroundItemInstance) -> Result<EntryId, OverlayError> {
        if item.world_id != self.world_id {
            return Err(OverlayError::World);
        }
        if item.channel_id != self.channel_id {
            return Err(OverlayError::Channel);
        }
        if item.ground.map_revision != self.map_revision {
            return Err(OverlayError::MapRevision);
        }
        let cell = decode_ground_cell(&item.ground.spatial_position)
            .map_err(|_| OverlayError::Position)?;
        let pos = match (
            u16::try_from(cell.x),
            u16::try_from(cell.y),
            i8::try_from(cell.z),
        ) {
            (Ok(x), Ok(y), Ok(floor)) => TilePos { x, y, floor },
            _ => return Err(OverlayError::Position),
        };
        self.top_level_count(pos)?;
        if self.ground.contains_key(&item.item_instance_id) {
            return Err(OverlayError::DuplicateItem);
        }
        let item_instance_id = item.item_instance_id;
        let item = AddedItem::Ground(Box::new(item));
        let cost = item_cost(&item);
        self.admit(self.tile_cost(pos) + cost, Admission::Durable)?;
        let id = self.push(pos, item, cost);
        self.ground.insert(item_instance_id, (pos, id));
        Ok(id)
    }

    fn push(&mut self, pos: TilePos, item: AddedItem, cost: usize) -> EntryId {
        let id = EntryId(self.next_entry);
        self.next_entry += 1;
        self.tiles
            .entry(pos)
            .or_default()
            .added
            .push(AddedEntry { id, item, cost });
        id
    }

    /// Removes the added entry `id` from the tile at `pos` and returns its item.
    pub fn remove(&mut self, pos: TilePos, id: EntryId) -> Result<AddedItem, OverlayError> {
        let tile = self.tiles.get_mut(&pos).ok_or(OverlayError::NoEntry)?;
        let at = tile
            .added
            .iter()
            .position(|entry| entry.id == id)
            .ok_or(OverlayError::NoEntry)?;
        let entry = tile.added.remove(at);
        // The refund covers the entry's slots only, so a tile kept by its hides or other entries
        // gives back the slots the removal freed rather than holding them uncharged.
        trim_added(&mut tile.added);
        self.used -= entry.cost;
        match &entry.item {
            AddedItem::Volatile {
                decays_at_ms: Some(decays_at_ms),
                ..
            } => {
                self.expiry
                    .remove(&(deadline_second(*decays_at_ms), id, pos));
            }
            AddedItem::Volatile { .. } => {}
            AddedItem::Ground(ground) => {
                self.ground.remove(&ground.item_instance_id);
                trim_table(&mut self.ground);
            }
        }
        self.release_tile(pos);
        Ok(entry.item)
    }

    /// Removes every volatile item whose decay second has come by `now_ms` (Unix ms) and
    /// returns where each was. Ticked each second, an item goes within 1 s of its decay and
    /// never before it.
    pub fn expire(&mut self, now_ms: u64) -> Vec<(TilePos, EntryId)> {
        let now_second = now_ms / 1000;
        let mut due: HashMap<TilePos, HashSet<EntryId>> = HashMap::new();
        let mut expired = Vec::new();
        while let Some(&(second, id, pos)) = self.expiry.first() {
            if second > now_second {
                break;
            }
            self.expiry.pop_first();
            due.entry(pos).or_default().insert(id);
            expired.push((pos, id));
        }
        // One pass per tile, so a crowded tile expires in linear time and what stays keeps its
        // presentation order.
        let mut removed = HashSet::new();
        for (pos, ids) in due {
            let Some(tile) = self.tiles.get_mut(&pos) else {
                continue;
            };
            let mut refund = 0;
            tile.added.retain(|entry| {
                let due =
                    ids.contains(&entry.id) && matches!(entry.item, AddedItem::Volatile { .. });
                if due {
                    refund += entry.cost;
                    removed.insert(entry.id);
                }
                !due
            });
            trim_added(&mut tile.added);
            self.used -= refund;
            self.release_tile(pos);
        }
        expired.retain(|(_, id)| removed.contains(id));
        expired
    }
}
