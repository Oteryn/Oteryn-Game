//! MAP-WIRE-2: the server side of domain 17 `WORLD_MAP_VIEW` (MAP-WIRE-1 contract §2-§4) for one
//! GameSession.
//!
//! - **Plan.** [`plan`] composes every tile of the window ([`crate::map::view`]), resolves
//!   `first_visible_floor` from the full composed stacks, cuts each stack and ranks the
//!   handle-bearing entries that are sent: the first `MAPW-RL-04` carry handles, the rest are
//!   `display_only`.
//! - **Snapshot or delta.** [`SessionMapView`] keeps the tiles it last sent. Each update plans the
//!   whole window again and sends every tile whose wire form changed, including a handle that a
//!   budget rerank gave or took, and clears every tile still in view that became empty. A step
//!   whose changes exceed `MAPW-RL-03`, a floor change, a teleport and a change of
//!   `(content_generation, bundle_digest, reset_epoch)` send a snapshot instead. A step with no
//!   changed tile sends the origin alone, and a changed `first_visible_floor` alone sends only
//!   the header.
//! - **Handles.** The handle-bearing entries within the budget are the session handle table's
//!   map view ([`SessionItemView::map_view`]); an entry that leaves it loses its handle. A movable
//!   base entry's handle is bound to `(bundle_digest, placement_key, reset_epoch)`.
//! - **Targets.** [`resolve_map_target`] resolves the 40-byte `WorldObjectTargetV1` of a
//!   `base_ordinal` entry (contract §4) and [`resolve_map_handle`] a handle-bearing entry. Command
//!   9 from a base-entry handle is `ITEM_MOVE_OUTCOME_NOT_SUPPORTED` until MAP-PICKUP-1
//!   ([`move_source_outcome`]).
//! - **Domain 1.** With capability 18, Ground items and corpses are carried only in map tiles
//!   ([`carried_in_domain_1`]).
//!
//! Capability 18 stays `offered: false` until MAP-CUTOVER-1, and no runtime holds a bundle World
//! yet, so only the tests reach this module.

#![cfg_attr(not(test), allow(dead_code))]

use super::item_view::{
    ItemHandleTable, ItemKey, ItemViewDelta, ItemViewError, ItemViewSnapshotDomain, SessionItemView,
};
use super::world_object::{UseDisposition, WorldObjectTarget};
use super::world_spatial::{ActorPosition, EntityKind};
use crate::map::overlay::{ChannelOverlay, TilePos};
use crate::map::view::{
    self, ComposedTile, EntrySource, MapEntryKey, MapFacts, RoofFacts, ViewError,
};
use oteryn_protocol_oteryn::item_view::ItemMoveOutcome;
use oteryn_protocol_oteryn::world_map::{
    DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1, MAX_DELTA_ENTRIES, MAX_MAP_VIEW_HANDLES, MapItem,
    MapOrigin, MapTile, MapViewHeader, SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
    STATE_DOMAIN_WORLD_MAP_VIEW, WorldMapViewDelta, WorldMapViewSnapshot, WorldMapWireError,
    encode_world_map_delta, encode_world_map_snapshot, in_window, tile_order,
};
use std::collections::BTreeMap;

/// The 40-byte placement of a `base_ordinal` entry's `USE`: the bundle digest and the
/// big-endian bundle placement key (contract §4 Carriage).
pub(crate) const MAP_TARGET_BYTES: usize = 40;

/// What a map view reads: the Channel overlay over its base, the per-entry facts, and the
/// binding of the view.
pub(crate) struct MapViewSource<'a, F> {
    pub(crate) overlay: &'a ChannelOverlay,
    pub(crate) facts: &'a F,
    /// The World's existing content generation.
    pub(crate) content_generation: [u8; 32],
    /// The Channel's current World reset epoch.
    pub(crate) reset_epoch: u64,
}

impl<F: MapFacts> MapViewSource<'_, F> {
    fn digest(&self) -> [u8; 32] {
        self.overlay.base().digest()
    }

    /// The handle-table key of a handle-bearing entry.
    fn item_key(&self, key: MapEntryKey) -> ItemKey {
        match key {
            MapEntryKey::Base(placement_key) => ItemKey::MapBase {
                bundle_digest: self.digest(),
                placement_key,
                reset_epoch: self.reset_epoch,
            },
            MapEntryKey::Added(id) => ItemKey::MapAdded(id.get()),
            MapEntryKey::Ground(id) => ItemKey::Instance(id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapViewError {
    /// The actor is not at a native map position.
    Position,
    View(ViewError),
    Items(ItemViewError),
    Wire(WorldMapWireError),
}

impl From<ViewError> for MapViewError {
    fn from(error: ViewError) -> Self {
        Self::View(error)
    }
}

impl From<ItemViewError> for MapViewError {
    fn from(error: ItemViewError) -> Self {
        Self::Items(error)
    }
}

impl From<WorldMapWireError> for MapViewError {
    fn from(error: WorldMapWireError) -> Self {
        Self::Wire(error)
    }
}

/// One sent entry before the handle table is consulted.
#[derive(Debug, Clone, Copy)]
struct PlannedEntry {
    source: EntrySource,
    item: MapItem,
    /// Within `MAPW-RL-04`; only for handle-bearing entries.
    in_budget: bool,
}

#[derive(Debug, Clone)]
struct PlannedTile {
    position: ActorPosition,
    entries: Vec<PlannedEntry>,
    more: bool,
    ground_speed: u16,
}

/// The whole window of one actor position, ready for the handle table.
#[derive(Debug, Clone)]
pub(crate) struct MapPlan {
    header: MapViewHeader,
    tiles: Vec<PlannedTile>,
    /// The handle-bearing entries within the budget, nearest first.
    keys: Vec<ItemKey>,
}

impl MapPlan {
    /// The handle-table keys within the budget, nearest first.
    pub(crate) fn keys(&self) -> &[ItemKey] {
        &self.keys
    }

    /// The wire tiles, with the handles `table` holds.
    fn tiles(
        &self,
        table: &ItemHandleTable,
        source: &MapViewSource<'_, impl MapFacts>,
    ) -> Result<BTreeMap<(i16, i32, i32), MapTile>, MapViewError> {
        let mut tiles = BTreeMap::new();
        for tile in &self.tiles {
            let mut items = Vec::with_capacity(tile.entries.len());
            for entry in &tile.entries {
                let origin = match entry.source {
                    EntrySource::Base {
                        ordinal,
                        placement_key,
                        movable: false,
                    } => MapOrigin::BaseOrdinal {
                        ordinal,
                        object_revision: source.facts.object_revision(placement_key),
                    },
                    _ if entry.in_budget => {
                        let key = source.item_key(entry.source.key());
                        let handle = table
                            .handle(&key)
                            .ok_or(MapViewError::Items(ItemViewError::LimitExceeded))?;
                        MapOrigin::Handle(handle)
                    }
                    _ => MapOrigin::DisplayOnly,
                };
                items.push(MapItem {
                    origin,
                    ..entry.item
                });
            }
            tiles.insert(
                tile_order(&tile.position),
                MapTile {
                    position: tile.position,
                    items,
                    more: tile.more,
                    ground_speed: tile.ground_speed,
                },
            );
        }
        Ok(tiles)
    }
}

fn position(pos: TilePos) -> ActorPosition {
    ActorPosition {
        x: i32::from(pos.x),
        y: i32::from(pos.y),
        floor: i16::from(pos.floor),
    }
}

/// The native tile position of an actor position; `None` off the native grid.
pub(crate) fn tile_pos(actor: ActorPosition) -> Option<TilePos> {
    let pos = TilePos {
        x: u16::try_from(actor.x).ok()?,
        y: u16::try_from(actor.y).ok()?,
        floor: i8::try_from(actor.floor).ok()?,
    };
    (-15..=0).contains(&pos.floor).then_some(pos)
}

/// Plans the window of an actor at `actor` (contract §2, §3).
pub(crate) fn plan<F: MapFacts>(
    source: &MapViewSource<'_, F>,
    actor: ActorPosition,
) -> Result<MapPlan, MapViewError> {
    let at = tile_pos(actor).ok_or(MapViewError::Position)?;
    let mut composed: BTreeMap<TilePos, Option<ComposedTile>> = BTreeMap::new();
    let window = view::window(at);
    for pos in window.iter().chain(&view::roof_positions(at)) {
        if !composed.contains_key(pos) {
            composed.insert(
                *pos,
                view::compose_tile(source.overlay, source.facts, *pos)?,
            );
        }
    }
    let first_visible_floor = view::first_visible_floor(at, |pos| {
        RoofFacts::of(composed.get(&pos).and_then(Option::as_ref))
    });

    let mut tiles = Vec::new();
    let mut ranked = Vec::new();
    for pos in &window {
        let Some(tile) = composed.get(pos).and_then(Option::as_ref) else {
            continue;
        };
        let (kept, more) = view::cut(&tile.entries);
        let bottom_skip = tile.entries.len() - kept.len();
        let mut entries = Vec::with_capacity(kept.len());
        for (index, entry) in kept.into_iter().enumerate() {
            let stack_index = if index == 0 { 0 } else { index + bottom_skip };
            if entry.source.handle_bearing() {
                ranked.push((
                    view::budget_order(at, *pos, stack_index),
                    tiles.len(),
                    entries.len(),
                ));
            }
            entries.push(PlannedEntry {
                source: entry.source,
                item: MapItem {
                    definition: entry.facts.definition,
                    count: entry.facts.count,
                    sub_type: entry.facts.sub_type,
                    origin: MapOrigin::DisplayOnly,
                    appearance_id: entry.facts.appearance_id,
                },
                in_budget: false,
            });
        }
        tiles.push(PlannedTile {
            position: position(*pos),
            entries,
            more,
            ground_speed: tile.ground_speed,
        });
    }
    ranked.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    let mut keys = Vec::with_capacity(ranked.len().min(MAX_MAP_VIEW_HANDLES));
    for (_, tile, entry) in ranked.into_iter().take(MAX_MAP_VIEW_HANDLES) {
        let entry = &mut tiles[tile].entries[entry];
        entry.in_budget = true;
        keys.push(source.item_key(entry.source.key()));
    }
    tiles.sort_unstable_by_key(|tile| tile_order(&tile.position));
    Ok(MapPlan {
        header: MapViewHeader {
            content_generation: source.content_generation,
            bundle_digest: source.digest(),
            reset_epoch: source.reset_epoch,
            origin: actor,
            first_visible_floor: i16::from(first_visible_floor),
        },
        tiles,
        keys,
    })
}

/// One update of domain 17.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MapUpdate {
    Snapshot(ItemViewSnapshotDomain),
    Delta(ItemViewDelta),
}

#[derive(Debug, Clone)]
struct SentView {
    header: MapViewHeader,
    tiles: BTreeMap<(i16, i32, i32), MapTile>,
}

/// One connection's view of domain 17: the tiles last sent and the domain revision.
#[derive(Debug, Default)]
pub(crate) struct SessionMapView {
    revision: u64,
    sent: Option<SentView>,
}

enum Planned {
    Snapshot(Vec<u8>),
    Delta(Vec<u8>),
    Nothing,
}

impl SessionMapView {
    /// The view of a new connection, whose revisions follow `last`.
    pub(crate) fn starting_after(last: u64) -> Self {
        Self {
            revision: last,
            sent: None,
        }
    }

    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    /// The join snapshot of domain 17 at `actor`.
    pub(crate) fn snapshot<F: MapFacts>(
        &mut self,
        items: &mut SessionItemView,
        source: &MapViewSource<'_, F>,
        actor: ActorPosition,
    ) -> Result<ItemViewSnapshotDomain, MapViewError> {
        self.sent = None;
        match self.update(items, source, actor)? {
            Some(MapUpdate::Snapshot(snapshot)) => Ok(snapshot),
            _ => Err(MapViewError::Wire(WorldMapWireError::Malformed)),
        }
    }

    /// The update after an actor move or a change of a tile in view: a snapshot, a delta, or
    /// nothing when no tile, origin or `first_visible_floor` changed. On an error nothing is
    /// sent and the view keeps its last sent state.
    pub(crate) fn update<F: MapFacts>(
        &mut self,
        items: &mut SessionItemView,
        source: &MapViewSource<'_, F>,
        actor: ActorPosition,
    ) -> Result<Option<MapUpdate>, MapViewError> {
        let plan = plan(source, actor)?;
        let sent = self.sent.as_ref();
        let (planned, tiles) = items.map_view(plan.keys(), |table| {
            let tiles = plan
                .tiles(table, source)
                .map_err(|_| ItemViewError::Encode)?;
            let planned = encode(sent, &plan.header, &tiles).map_err(|_| ItemViewError::Encode)?;
            Ok((planned, tiles))
        })?;
        let update = match planned {
            Planned::Nothing => None,
            Planned::Snapshot(payload) => Some(MapUpdate::Snapshot(ItemViewSnapshotDomain {
                domain_id: STATE_DOMAIN_WORLD_MAP_VIEW,
                revision: advance(&mut self.revision)?,
                snapshot_type: SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
                payload,
            })),
            Planned::Delta(payload) => {
                let from = self.revision;
                Some(MapUpdate::Delta(ItemViewDelta {
                    domain_id: STATE_DOMAIN_WORLD_MAP_VIEW,
                    from,
                    to: advance(&mut self.revision)?,
                    delta_type: DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1,
                    payload,
                }))
            }
        };
        self.sent = Some(SentView {
            header: plan.header,
            tiles,
        });
        Ok(update)
    }
}

fn advance(revision: &mut u64) -> Result<u64, MapViewError> {
    *revision = revision
        .checked_add(1)
        .ok_or(MapViewError::Items(ItemViewError::Exhausted))?;
    Ok(*revision)
}

/// Whether a delta from `sent` can carry the move to `header`: the same binding, the same floor
/// and at most one step (contract §3 Origin).
fn delta_applies(sent: &MapViewHeader, header: &MapViewHeader) -> bool {
    let (from, to) = (sent.origin, header.origin);
    sent.content_generation == header.content_generation
        && sent.bundle_digest == header.bundle_digest
        && sent.reset_epoch == header.reset_epoch
        && from.floor == to.floor
        && i64::from(from.x).abs_diff(i64::from(to.x)) <= 1
        && i64::from(from.y).abs_diff(i64::from(to.y)) <= 1
}

fn encode(
    sent: Option<&SentView>,
    header: &MapViewHeader,
    tiles: &BTreeMap<(i16, i32, i32), MapTile>,
) -> Result<Planned, WorldMapWireError> {
    let snapshot = || {
        encode_world_map_snapshot(&WorldMapViewSnapshot {
            header: *header,
            tiles: tiles.values().cloned().collect(),
        })
        .map(Planned::Snapshot)
    };
    let Some(sent) = sent.filter(|sent| delta_applies(&sent.header, header)) else {
        return snapshot();
    };
    let mut changed = Vec::new();
    for (order, tile) in tiles {
        if sent.tiles.get(order) != Some(tile) {
            if changed.len() == MAX_DELTA_ENTRIES {
                return snapshot();
            }
            changed.push(tile.clone());
        }
    }
    let cleared: Vec<ActorPosition> = sent
        .tiles
        .iter()
        .filter(|(order, tile)| {
            !tiles.contains_key(order) && in_window(header.origin, tile.position)
        })
        .map(|(_, tile)| tile.position)
        .collect();
    if changed.len() + cleared.len() > MAX_DELTA_ENTRIES {
        return snapshot();
    }
    if changed.is_empty()
        && cleared.is_empty()
        && sent.header.origin == header.origin
        && sent.header.first_visible_floor == header.first_visible_floor
    {
        return Ok(Planned::Nothing);
    }
    encode_world_map_delta(
        &sent.header,
        &WorldMapViewDelta {
            header: *header,
            tiles: changed,
            cleared,
        },
    )
    .map(Planned::Delta)
}

/// A resolved 40-byte map target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MapObjectTarget<C> {
    pub(crate) pos: TilePos,
    pub(crate) ordinal: u8,
    pub(crate) placement_key: u64,
    /// The canonical key of a RewardClaim placement bound to the entry: the `source_placement`
    /// of the MINT and audit.
    pub(crate) canonical: Option<C>,
}

/// Why a map target does not resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapTargetRefusal {
    /// Another bundle digest: `STATE_REVISION_MISMATCH`, and the client resyncs.
    StateRevisionMismatch,
    /// A `USE` outcome: `NOTHING_TO_USE` or `STALE`.
    Use(UseDisposition),
}

/// Whether top-level base entry `ordinal` of `pos` exists and the overlay does not hide it.
fn unhidden_base_entry(overlay: &ChannelOverlay, pos: TilePos, ordinal: u8) -> bool {
    let Some(tile) = overlay.base().tile(pos.x, pos.y, pos.floor) else {
        return false;
    };
    let exists = tile.depths().iter().filter(|depth| **depth == 0).count() > usize::from(ordinal);
    exists
        && !overlay
            .tile(pos)
            .is_some_and(|view| view.is_hidden(ordinal))
}

/// Resolves the 40-byte `WorldObjectTargetV1` of a bundle World (contract §4): the active digest,
/// an unhidden base entry, its `expected_revision`, then its RewardClaim binding through
/// `binding`, which writes nothing.
pub(crate) fn resolve_map_target<F: MapFacts, C>(
    source: &MapViewSource<'_, F>,
    target: &WorldObjectTarget,
    binding: impl FnOnce(u64) -> Option<C>,
) -> Result<MapObjectTarget<C>, MapTargetRefusal> {
    let nothing = MapTargetRefusal::Use(UseDisposition::NothingToUse);
    let placement: &[u8; MAP_TARGET_BYTES] = target
        .placement
        .as_slice()
        .try_into()
        .map_err(|_| nothing)?;
    let (digest, key) = placement.split_at(32);
    if digest != source.digest() {
        return Err(MapTargetRefusal::StateRevisionMismatch);
    }
    let placement_key = u64::from_be_bytes(key.try_into().map_err(|_| nothing)?);
    let (pos, ordinal) = view::placement_of(placement_key).ok_or(nothing)?;
    if !unhidden_base_entry(source.overlay, pos, ordinal) {
        return Err(nothing);
    }
    if target.expected_revision != source.facts.object_revision(placement_key) {
        return Err(MapTargetRefusal::Use(UseDisposition::StaleState));
    }
    Ok(MapObjectTarget {
        pos,
        ordinal,
        placement_key,
        canonical: binding(placement_key),
    })
}

/// The map entry a handle-bearing key names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapHandleTarget {
    Base {
        pos: TilePos,
        ordinal: u8,
        placement_key: u64,
    },
    Added {
        pos: TilePos,
        id: u64,
    },
    Ground {
        pos: TilePos,
        item_instance_id: [u8; 16],
    },
}

/// Resolves the map entry of a handle's key (contract §3 Move source); `STALE` when the bundle,
/// the reset epoch or the overlay no longer holds it.
pub(crate) fn resolve_map_handle<F: MapFacts>(
    source: &MapViewSource<'_, F>,
    key: ItemKey,
) -> Result<MapHandleTarget, UseDisposition> {
    let stale = UseDisposition::StaleState;
    match key {
        ItemKey::MapBase {
            bundle_digest,
            placement_key,
            reset_epoch,
        } => {
            if bundle_digest != source.digest() || reset_epoch != source.reset_epoch {
                return Err(stale);
            }
            let (pos, ordinal) = view::placement_of(placement_key).ok_or(stale)?;
            if !unhidden_base_entry(source.overlay, pos, ordinal) {
                return Err(stale);
            }
            Ok(MapHandleTarget::Base {
                pos,
                ordinal,
                placement_key,
            })
        }
        ItemKey::MapAdded(id) => source
            .overlay
            .tiles()
            .find_map(|(pos, view)| {
                view.added()
                    .iter()
                    .any(|entry| entry.id().get() == id)
                    .then_some(MapHandleTarget::Added { pos, id })
            })
            .ok_or(stale),
        ItemKey::Instance(item_instance_id) => source
            .overlay
            .ground_entry(&item_instance_id)
            .map(|(pos, _)| MapHandleTarget::Ground {
                pos,
                item_instance_id,
            })
            .ok_or(stale),
        ItemKey::Entity(_) => Err(stale),
    }
}

/// Command 9 from a base-entry handle (contract §3 Move source): `NOT_SUPPORTED`, writing
/// nothing, until MAP-PICKUP-1. `None` for every other source, which keeps its path.
pub(crate) const fn move_source_outcome(key: &ItemKey) -> Option<ItemMoveOutcome> {
    match key {
        ItemKey::MapBase { .. } => Some(ItemMoveOutcome::NotSupported),
        _ => None,
    }
}

/// Whether a domain-1 object of `kind` is carried in domain 1: with capability 18, Ground items
/// and corpses are carried only in map tiles (contract §3).
pub(crate) const fn carried_in_domain_1(kind: EntityKind, map_view: bool) -> bool {
    !(map_view && matches!(kind, EntityKind::GroundItem | EntityKind::Corpse))
}

#[cfg(test)]
#[path = "world_map_tests.rs"]
mod tests;
