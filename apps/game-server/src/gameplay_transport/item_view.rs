//! ITEM-VIEW-1b: the server side of ITEM-MOVE-WIRE-0 §4 for one GameSession.
//!
//! - **Handles (§4.1).** Every item a session sees carries a handle from one counter per
//!   `GameSessionId`: monotonic, never reused. A handle is live while its item is in at least one
//!   of the session's views (domain 1 objects, domain 9, domain 11); when it leaves every view the
//!   entry is dropped, and an item that returns gets a new handle. Live handles are bounded by
//!   `ITEMV0-RL-03`. A handle is only a number from the counter: it never exposes an
//!   ItemInstanceId, placement key or row, which stay in the server-internal [`ItemKey`].
//! - **Views and revisions (§4.2).** Domain 9 shows the main backpack slot and its direct entries
//!   from `read_character_backpack`; domain 11 the one open corpse. Each delta carries the whole
//!   view. Revisions are monotonic per GameSession: every snapshot is above any revision the
//!   session has seen, and a revision advances before its write, so one that may have reached the
//!   client is never reused.
//! - **Continuity.** [`ItemViewContinuity`] (the handle counter, the two high-water revisions and
//!   the open corpse) travels in the session continuity on reconnect and channel transfer. The
//!   table itself does not: the next snapshot reissues fresh handles from the carried counter,
//!   and every older handle resolves to `STALE`.
//! - **Opening (§4.3).** `USE` with an item target opens a corpse in reach and writes nothing;
//!   [`corpse_open`] decides reach and the closing triggers.
//! - **Container tree (BAGS-WIRE-1).** With capability 14 the table has a fourth view, the
//!   containers and entries of domain 14 ([`super::container_view`]), and its bound is
//!   `ITEMV0-RL-03-CONTAINER-TREE`. Domain 14 carries its own high-water revision.
//! - **Map view (MAP-WIRE-2).** With capability 18 the table has a fifth view, the handle-bearing
//!   entries of domain 17 within `MAPW-RL-04` ([`super::world_map`]), and its bound adds 1,024:
//!   `ITEMV0-RL-03-MAP-VIEW`.
//! - **Taking (ITEM-MOVE-1).** Command 9 takes an entry of the open corpse ([`super::item_move`]);
//!   after the commit the corpse's domain 11 delta and the domain 9 delta follow.

#![cfg_attr(not(test), allow(dead_code))]

#[path = "../interaction/corpse_open.rs"]
pub(crate) mod corpse_open;

use super::container_view::ViewCommandWindow;
use super::world_spatial::{
    ActorPosition, ENTITY_IDENTITY_BYTES, EntityDetail, WorldSpatialEntity,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::CharacterBackpack;
pub(crate) use corpse_open::{CloseTrigger, OpenDecision, UseItemTarget};
use oteryn_protocol_oteryn::container_tree::{
    DELTA_TYPE_CONTAINER_VIEWS_V1, MAX_LIVE_ITEM_HANDLES_CONTAINER_TREE,
    SNAPSHOT_TYPE_CONTAINER_VIEWS_V1, STATE_DOMAIN_CONTAINER_VIEWS,
};
use oteryn_protocol_oteryn::item_view::{
    CharacterInventory, DELTA_TYPE_CHARACTER_INVENTORY_V1, DELTA_TYPE_OPEN_CONTAINER_V1, ItemEntry,
    ItemHandle, MAX_LIVE_ITEM_HANDLES, OpenContainer, SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
    SNAPSHOT_TYPE_OPEN_CONTAINER_V1, STATE_DOMAIN_CHARACTER_INVENTORY, STATE_DOMAIN_OPEN_CONTAINER,
    encode_character_inventory, encode_open_container,
};
use oteryn_protocol_oteryn::world_map::MAX_MAP_VIEW_HANDLES;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::{BuildHasherDefault, Hasher};
use std::num::NonZeroU32;
use tokio::time::Instant;

/// The server-internal identity of one viewed item. Never on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ItemKey {
    /// A domain-1 object (a corpse or a ground item) by its channel-wide `EntityIdentity`.
    Entity([u8; ENTITY_IDENTITY_BYTES]),
    /// A live item instance: the main backpack, its entries and a corpse's entries, and a Ground
    /// item of the map view.
    Instance([u8; 16]),
    /// A movable base entry of the map view (MAP-WIRE-1 §3 Move source), never an ItemInstance.
    MapBase {
        bundle_digest: [u8; 32],
        placement_key: u64,
        reset_epoch: u64,
    },
    /// An overlay-added volatile item of the map view, by its overlay entry id.
    MapAdded(u64),
}

/// One item of a view, with its server-internal key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ViewItem {
    pub(crate) key: ItemKey,
    pub(crate) item_definition_ref: NonZeroU32,
    pub(crate) count: NonZeroU32,
    pub(crate) sub_type: u32,
}

/// Domain 9's content: the main backpack slot and its direct entries in display order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct InventoryItems {
    pub(crate) main_backpack: Option<ViewItem>,
    pub(crate) entries: Vec<ViewItem>,
}

/// Domain 9's content from `read_character_backpack` (`durability/item_transfer.rs`).
/// `definition_ref` names each stored definition's wire `item_definition_ref`; `None` when a
/// definition has none or a stack is empty, which fails closed.
pub(crate) fn inventory_from_backpack(
    backpack: Option<&CharacterBackpack>,
    definition_ref: impl Fn(&TypedDefinitionRef) -> Option<NonZeroU32>,
) -> Option<InventoryItems> {
    let Some(backpack) = backpack else {
        return Some(InventoryItems::default());
    };
    let item = |item: &crate::durability::item_transfer::InventoryItem| {
        Some(ViewItem {
            key: ItemKey::Instance(item.item_instance_id),
            item_definition_ref: definition_ref(&item.definition)?,
            count: NonZeroU32::new(item.quantity)?,
            sub_type: 0,
        })
    };
    Some(InventoryItems {
        main_backpack: Some(item(&backpack.backpack)?),
        entries: backpack
            .entries
            .iter()
            .map(|entry| item(&entry.item))
            .collect::<Option<_>>()?,
    })
}

/// What the Channel owner observes for the item target of one `USE`: the actor's position and the
/// target, read together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemTargetObservation {
    pub(crate) actor: ActorPosition,
    pub(crate) target: UseItemTarget<Vec<ViewItem>>,
}

/// The item view part of one GameSession's continuity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ItemViewContinuity {
    /// The last handle issued in this GameSession (0: none yet).
    pub(crate) handle_counter: u64,
    /// The highest domain 9 revision the session may have seen.
    pub(crate) inventory_revision: u64,
    /// The highest domain 11 revision the session may have seen.
    pub(crate) container_revision: u64,
    /// The open corpse. A reconnect snapshot reopens it only if it is still in reach.
    pub(crate) open_corpse: Option<ItemKey>,
    /// The highest domain 14 revision the session may have seen (capability 14). The views
    /// themselves close on every reconnect and transfer.
    pub(crate) views_revision: u64,
    /// `BAGS0-RL-04`: the GameSession's recent view commands (capability 14).
    pub(crate) view_commands: ViewCommandWindow,
}

impl ItemViewContinuity {
    /// A channel transfer closes the open corpse (§4.3) and carries the rest.
    pub(crate) const fn across_channel_transfer(self) -> Self {
        Self {
            open_corpse: None,
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemViewError {
    /// More than `ITEMV0-RL-03` (with capability 14, `ITEMV0-RL-03-CONTAINER-TREE`) live handles;
    /// the view is left unchanged.
    LimitExceeded,
    /// The handle counter or a revision would wrap.
    Exhausted,
    /// The view does not encode (a bound or a repeated item).
    Encode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum View {
    Spatial = 0,
    Inventory = 1,
    Container = 2,
    /// Domain 14, capability 14 only.
    Tree = 3,
    /// Domain 17, capability 18 only.
    Map = 4,
}

/// A fixed, unkeyed multiply-rotate hasher (the FxHash step) for the handle table's
/// server-internal keys: handles and keys are issued by the server, never chosen by a client, and
/// the table is bounded by `ITEMV0-RL-03-*`, so a keyed hasher buys nothing here.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct KeyHasher(u64);

impl KeyHasher {
    const fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

impl Hasher for KeyHasher {
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.add(u64::from_le_bytes(word));
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.add(u64::from(value));
    }

    fn write_u32(&mut self, value: u32) {
        self.add(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }

    fn finish(&self) -> u64 {
        // The multiply leaves the low bits weakest; the table indexes by them.
        self.0.rotate_left(26)
    }
}

type KeyMap<K, V> = HashMap<K, V, BuildHasherDefault<KeyHasher>>;

/// One slot of the handle table: a live key, its handle and the views that hold it, one bit per
/// [`View`]. A free slot holds no view.
#[derive(Debug, Clone, Copy)]
struct Slot {
    key: ItemKey,
    handle: ItemHandle,
    views: u8,
}

/// Marks a key of the next content while [`ItemHandleTable::replace`] runs.
const NEXT: u8 = 1 << 7;

/// The session's handle table (§4.1): a slot per live key, found by key and by handle.
#[derive(Debug, Clone, Default)]
pub(crate) struct ItemHandleTable {
    last: u64,
    /// The slots of each view, without repeats, in the order the keys were first given.
    views: [Vec<u32>; 5],
    /// Capability 14 is selected: the bound is `ITEMV0-RL-03-CONTAINER-TREE`.
    container_tree: bool,
    /// Capability 18 is selected: the bound adds `MAPW-RL-04`.
    map_view: bool,
    slots: Vec<Slot>,
    free: Vec<u32>,
    by_key: KeyMap<ItemKey, u32>,
    by_handle: KeyMap<ItemHandle, u32>,
}

impl ItemHandleTable {
    /// An empty table whose next handle follows `last`.
    pub(crate) fn starting_after(last: u64) -> Self {
        Self {
            last,
            ..Self::default()
        }
    }

    /// The table of a session that selected capability 14.
    pub(crate) const fn with_container_tree(mut self) -> Self {
        self.container_tree = true;
        self
    }

    /// The table of a session that selected capability 18.
    pub(crate) const fn with_map_view(mut self) -> Self {
        self.map_view = true;
        self
    }

    /// The bound of this session's live handles.
    pub(crate) const fn limit(&self) -> usize {
        let base = if self.container_tree {
            MAX_LIVE_ITEM_HANDLES_CONTAINER_TREE
        } else {
            MAX_LIVE_ITEM_HANDLES
        };
        if self.map_view {
            base + MAX_MAP_VIEW_HANDLES
        } else {
            base
        }
    }

    pub(crate) const fn last_issued(&self) -> u64 {
        self.last
    }

    pub(crate) fn live(&self) -> usize {
        self.by_key.len()
    }

    /// The item a handle names; `None` is `STALE`.
    pub(crate) fn resolve(&self, handle: ItemHandle) -> Option<ItemKey> {
        let slot = *self.by_handle.get(&handle)?;
        Some(self.slots[slot as usize].key)
    }

    pub(crate) fn handle(&self, key: &ItemKey) -> Option<ItemHandle> {
        let slot = *self.by_key.get(key)?;
        Some(self.slots[slot as usize].handle)
    }

    /// The keys of `view` in key order.
    fn keys_of(&self, view: View) -> Vec<ItemKey> {
        let mut keys: Vec<ItemKey> = self.views[view as usize]
            .iter()
            .map(|slot| self.slots[*slot as usize].key)
            .collect();
        keys.sort_unstable();
        keys
    }

    /// Makes `keys` the content of `view`: an item new to every view gets the next handle, in the
    /// given order, and one that left every view is dropped. Over the bound nothing changes.
    pub(crate) fn replace(&mut self, view: View, keys: &[ItemKey]) -> Result<(), ItemViewError> {
        self.replace_undoable(view, keys).map(drop)
    }

    /// [`Self::replace`] that also returns what [`Self::rollback`] needs to take it back.
    fn replace_undoable(
        &mut self,
        view: View,
        keys: &[ItemKey],
    ) -> Result<ReplaceUndo, ItemViewError> {
        let index = view as usize;
        let bit = 1 << index;
        let mut undo = ReplaceUndo {
            view: index,
            last: self.last,
            previous: Vec::new(),
            removed: Vec::new(),
            issued: Vec::new(),
        };
        // Mark every key of the next content; a key new to every view gets the next handle.
        let mut next = Vec::with_capacity(keys.len());
        let mut exhausted = false;
        for key in keys {
            match self.by_key.entry(*key) {
                Entry::Occupied(entry) => {
                    let slot = *entry.get();
                    let views = &mut self.slots[slot as usize].views;
                    if *views & NEXT == 0 {
                        *views |= NEXT;
                        next.push(slot);
                    }
                }
                Entry::Vacant(entry) => {
                    let Some(handle) = self.last.checked_add(1).and_then(ItemHandle::new) else {
                        exhausted = true;
                        break;
                    };
                    self.last = handle.get();
                    let fresh = Slot {
                        key: *key,
                        handle,
                        views: NEXT,
                    };
                    let slot = match self.free.pop() {
                        Some(slot) => {
                            self.slots[slot as usize] = fresh;
                            slot
                        }
                        None => {
                            self.slots.push(fresh);
                            (self.slots.len() - 1) as u32
                        }
                    };
                    entry.insert(slot);
                    self.by_handle.insert(handle, slot);
                    undo.issued.push(slot);
                    next.push(slot);
                }
            }
        }
        if exhausted {
            for slot in &next {
                self.slots[*slot as usize].views &= !NEXT;
            }
            self.drop_issued(&undo.issued);
            self.last = undo.last;
            return Err(if self.live_after(index, keys) > self.limit() {
                ItemViewError::LimitExceeded
            } else {
                ItemViewError::Exhausted
            });
        }
        // A key that left this view and is in no other is dropped.
        for slot in &self.views[index] {
            let entry = &mut self.slots[*slot as usize];
            if entry.views & NEXT != 0 {
                continue;
            }
            entry.views &= !bit;
            if entry.views == 0 {
                self.by_key.remove(&entry.key);
                self.by_handle.remove(&entry.handle);
                self.free.push(*slot);
                undo.removed.push(*slot);
            }
        }
        for slot in &next {
            let views = &mut self.slots[*slot as usize].views;
            *views = *views & !NEXT | bit;
        }
        undo.previous = std::mem::replace(&mut self.views[index], next);
        if self.by_key.len() > self.limit() {
            self.rollback(undo);
            return Err(ItemViewError::LimitExceeded);
        }
        Ok(undo)
    }

    /// The live handles after `keys` became the content of view `index`.
    fn live_after(&self, index: usize, keys: &[ItemKey]) -> usize {
        let mut live: KeyMap<ItemKey, ()> = keys.iter().map(|key| (*key, ())).collect();
        for (other, slots) in self.views.iter().enumerate() {
            if other != index {
                for slot in slots {
                    live.insert(self.slots[*slot as usize].key, ());
                }
            }
        }
        live.len()
    }

    /// Frees the slots `issued` holds, newest last.
    fn drop_issued(&mut self, issued: &[u32]) {
        for slot in issued.iter().rev() {
            let entry = &mut self.slots[*slot as usize];
            entry.views = 0;
            self.by_key.remove(&entry.key);
            self.by_handle.remove(&entry.handle);
            self.free.push(*slot);
        }
    }

    /// Takes back the [`Self::replace_undoable`] that made `undo`, which must be the last change.
    fn rollback(&mut self, undo: ReplaceUndo) {
        let bit = 1 << undo.view;
        for slot in &self.views[undo.view] {
            self.slots[*slot as usize].views &= !bit;
        }
        // The dropped slots are the last freed and still hold their key and handle.
        self.free.truncate(self.free.len() - undo.removed.len());
        for slot in &undo.removed {
            let entry = &self.slots[*slot as usize];
            self.by_key.insert(entry.key, *slot);
            self.by_handle.insert(entry.handle, *slot);
        }
        self.drop_issued(&undo.issued);
        for slot in &undo.previous {
            self.slots[*slot as usize].views |= bit;
        }
        self.views[undo.view] = undo.previous;
        self.last = undo.last;
    }
}

/// What taking back one `replace` needs.
struct ReplaceUndo {
    view: usize,
    last: u64,
    previous: Vec<u32>,
    removed: Vec<u32>,
    issued: Vec<u32>,
}

/// One domain of a snapshot: id, revision, snapshot type and payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemViewSnapshotDomain {
    pub(crate) domain_id: u32,
    pub(crate) revision: u64,
    pub(crate) snapshot_type: u32,
    pub(crate) payload: Vec<u8>,
}

/// One whole-view delta, emitted after the change it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemViewDelta {
    pub(crate) domain_id: u32,
    pub(crate) from: u64,
    pub(crate) to: u64,
    pub(crate) delta_type: u32,
    pub(crate) payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenCorpse {
    key: ItemKey,
    position: ActorPosition,
}

/// One connection's view of domains 9 and 11 and its handle table.
#[derive(Debug)]
pub(crate) struct SessionItemView {
    table: ItemHandleTable,
    inventory_revision: u64,
    container_revision: u64,
    views_revision: u64,
    view_commands: ViewCommandWindow,
    inventory: InventoryItems,
    open: Option<OpenCorpse>,
    open_entries: Vec<ViewItem>,
    /// The open corpse carried from the previous connection until the snapshot decides it.
    carried_open: Option<ItemKey>,
}

impl SessionItemView {
    /// The view of a new connection: no live handle, the counter and revisions carried.
    pub(crate) fn resume(continuity: ItemViewContinuity) -> Self {
        Self {
            table: ItemHandleTable::starting_after(continuity.handle_counter),
            inventory_revision: continuity.inventory_revision,
            container_revision: continuity.container_revision,
            views_revision: continuity.views_revision,
            view_commands: continuity.view_commands,
            inventory: InventoryItems::default(),
            open: None,
            open_entries: Vec::new(),
            carried_open: continuity.open_corpse,
        }
    }

    pub(crate) fn continuity(&self) -> ItemViewContinuity {
        ItemViewContinuity {
            handle_counter: self.table.last_issued(),
            inventory_revision: self.inventory_revision,
            container_revision: self.container_revision,
            open_corpse: self.open.map(|open| open.key).or(self.carried_open),
            views_revision: self.views_revision,
            view_commands: self.view_commands,
        }
    }

    /// `BAGS0-RL-04`: whether one more view command of this GameSession fits the window.
    pub(crate) fn admit_view_command(&mut self, now: Instant) -> bool {
        self.view_commands.admit(now)
    }

    /// Capability 14 is selected: the handle bound becomes `ITEMV0-RL-03-CONTAINER-TREE`.
    pub(crate) fn with_container_tree(mut self) -> Self {
        self.table = self.table.with_container_tree();
        self
    }

    /// Capability 18 is selected: the handle bound adds `MAPW-RL-04` (`ITEMV0-RL-03-MAP-VIEW`).
    pub(crate) fn with_map_view(mut self) -> Self {
        self.table = self.table.with_map_view();
        self
    }

    /// Makes `keys` the handle-bearing entries of domain 17 and returns what `encode` builds from
    /// the updated table. When the keys are over the bound or the view does not encode, the table
    /// is left unchanged.
    pub(crate) fn map_view<T>(
        &mut self,
        keys: &[ItemKey],
        encode: impl FnOnce(&ItemHandleTable) -> Result<T, ItemViewError>,
    ) -> Result<T, ItemViewError> {
        let undo = self.table.replace_undoable(View::Map, keys)?;
        encode(&self.table).inspect_err(|_| self.table.rollback(undo))
    }

    /// The domain 14 snapshot of a new connection: no view open, above any revision the session
    /// has seen.
    pub(crate) fn views_snapshot(&mut self) -> Result<ItemViewSnapshotDomain, ItemViewError> {
        self.table.replace(View::Tree, &[])?;
        Ok(ItemViewSnapshotDomain {
            domain_id: STATE_DOMAIN_CONTAINER_VIEWS,
            revision: advance(&mut self.views_revision)?,
            snapshot_type: SNAPSHOT_TYPE_CONTAINER_VIEWS_V1,
            payload: Vec::new(),
        })
    }

    /// Makes `keys` the content of domain 14 and returns its whole-view delta, with the payload
    /// `encode` builds from the updated table. When the keys are over the bound or the views do
    /// not encode, nothing changes.
    pub(crate) fn tree_delta(
        &mut self,
        keys: &[ItemKey],
        encode: impl FnOnce(&ItemHandleTable) -> Result<Vec<u8>, ItemViewError>,
    ) -> Result<ItemViewDelta, ItemViewError> {
        let previous = self.table.keys_of(View::Tree);
        self.table.replace(View::Tree, keys)?;
        let payload = match encode(&self.table) {
            Ok(payload) => payload,
            Err(error) => {
                let _ = self.table.replace(View::Tree, &previous);
                return Err(error);
            }
        };
        let from = self.views_revision;
        let to = advance(&mut self.views_revision)?;
        Ok(ItemViewDelta {
            domain_id: STATE_DOMAIN_CONTAINER_VIEWS,
            from,
            to,
            delta_type: DELTA_TYPE_CONTAINER_VIEWS_V1,
            payload,
        })
    }

    pub(crate) const fn table(&self) -> &ItemHandleTable {
        &self.table
    }

    /// The corpse carried from the previous connection, which the snapshot reopens only if it is
    /// still in reach.
    pub(crate) const fn carried_open_corpse(&self) -> Option<ItemKey> {
        self.carried_open
    }

    /// The item a handle names; `None` is `STALE`.
    pub(crate) fn resolve(&self, handle: ItemHandle) -> Option<ItemKey> {
        self.table.resolve(handle)
    }

    /// The domain 1 objects of a spatial snapshot or delta: each gets its handle (§4.1).
    pub(crate) fn attach_spatial_handles(
        &mut self,
        entities: &mut [WorldSpatialEntity],
    ) -> Result<(), ItemViewError> {
        let keys: Vec<ItemKey> = entities
            .iter()
            .filter(|entity| matches!(entity.detail, EntityDetail::Object { .. }))
            .map(|entity| ItemKey::Entity(entity.entity.identity))
            .collect();
        self.table.replace(View::Spatial, &keys)?;
        for entity in entities {
            let key = ItemKey::Entity(entity.entity.identity);
            if let EntityDetail::Object { item_handle, .. } = &mut entity.detail {
                *item_handle = self.table.handle(&key);
            }
        }
        Ok(())
    }

    /// The domain 9 and 11 snapshot of a new connection, each above any revision the session has
    /// seen. `reopen` is the observation of [`Self::carried_open_corpse`]: it is shown again only
    /// while it is a corpse in reach; otherwise domain 11 is empty.
    pub(crate) fn snapshot(
        &mut self,
        inventory: InventoryItems,
        reopen: Option<ItemTargetObservation>,
    ) -> Result<[ItemViewSnapshotDomain; 2], ItemViewError> {
        let carried = self.carried_open.take();
        let inventory_payload = self.set_inventory(inventory)?;
        let (open, entries) = match (carried, reopen) {
            (
                Some(key),
                Some(ItemTargetObservation {
                    actor,
                    target: UseItemTarget::Corpse { position, contents },
                }),
            ) if corpse_open::within_reach(actor, position) => {
                (Some(OpenCorpse { key, position }), contents)
            }
            _ => (None, Vec::new()),
        };
        let container_payload = self.set_container(open, entries)?;
        let inventory_revision = advance(&mut self.inventory_revision)?;
        let container_revision = advance(&mut self.container_revision)?;
        Ok([
            ItemViewSnapshotDomain {
                domain_id: STATE_DOMAIN_CHARACTER_INVENTORY,
                revision: inventory_revision,
                snapshot_type: SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                payload: inventory_payload,
            },
            ItemViewSnapshotDomain {
                domain_id: STATE_DOMAIN_OPEN_CONTAINER,
                revision: container_revision,
                snapshot_type: SNAPSHOT_TYPE_OPEN_CONTAINER_V1,
                payload: container_payload,
            },
        ])
    }

    /// The domain 9 delta after a durable commit that may have changed the backpack; `None` when
    /// the view is unchanged.
    pub(crate) fn inventory_committed(
        &mut self,
        inventory: InventoryItems,
    ) -> Result<Option<ItemViewDelta>, ItemViewError> {
        if inventory == self.inventory {
            return Ok(None);
        }
        let payload = self.set_inventory(inventory)?;
        let from = self.inventory_revision;
        let to = advance(&mut self.inventory_revision)?;
        Ok(Some(ItemViewDelta {
            domain_id: STATE_DOMAIN_CHARACTER_INVENTORY,
            from,
            to,
            delta_type: DELTA_TYPE_CHARACTER_INVENTORY_V1,
            payload,
        }))
    }

    /// The open corpse when `key` is one of its entries; `None` for any other item.
    pub(crate) fn open_corpse_of(&self, key: &ItemKey) -> Option<ItemKey> {
        let open = self.open?;
        self.open_entries
            .iter()
            .any(|item| item.key == *key)
            .then_some(open.key)
    }

    /// The domain 11 delta after a durable commit that may have changed the open corpse,
    /// decided on the Channel owner's observation of it: the corpse in reach shows its contents
    /// now; gone, out of reach or not observed, it closes. `None` when nothing is open or the view
    /// is unchanged.
    pub(crate) fn open_corpse_committed(
        &mut self,
        observation: Option<ItemTargetObservation>,
    ) -> Result<Option<ItemViewDelta>, ItemViewError> {
        let Some(open) = self.open else {
            return Ok(None);
        };
        let (next, entries) = match observation {
            Some(ItemTargetObservation {
                actor,
                target: UseItemTarget::Corpse { position, contents },
            }) if corpse_open::within_reach(actor, position) => (
                Some(OpenCorpse {
                    key: open.key,
                    position,
                }),
                contents,
            ),
            _ => (None, Vec::new()),
        };
        if next == self.open && entries == self.open_entries {
            return Ok(None);
        }
        self.container_delta(next, entries).map(Some)
    }

    /// One `USE` with an item target (§4.3), decided on the Channel owner's observation of the key
    /// the handle resolved to. Opening another corpse replaces the open one; the domain 11 delta
    /// follows the `COMMITTED` result. Nothing is written.
    pub(crate) fn open(
        &mut self,
        key: ItemKey,
        observation: ItemTargetObservation,
    ) -> Result<(OpenDecision, Option<ItemViewDelta>), ItemViewError> {
        let decision = corpse_open::decide_open(observation.actor, &observation.target);
        let UseItemTarget::Corpse { position, contents } = observation.target else {
            return Ok((decision, None));
        };
        if decision != OpenDecision::Open {
            return Ok((decision, None));
        }
        let delta = self.container_delta(Some(OpenCorpse { key, position }), contents)?;
        Ok((decision, Some(delta)))
    }

    /// Closes the open corpse when `trigger` closes it (§4.3): the domain 11 delta to an empty
    /// view, or `None` when nothing closed.
    pub(crate) fn close(
        &mut self,
        trigger: &CloseTrigger<ItemKey>,
    ) -> Result<Option<ItemViewDelta>, ItemViewError> {
        let Some(open) = self.open else {
            return Ok(None);
        };
        if !corpse_open::closes(&open.key, open.position, trigger) {
            return Ok(None);
        }
        self.container_delta(None, Vec::new()).map(Some)
    }

    fn container_delta(
        &mut self,
        open: Option<OpenCorpse>,
        entries: Vec<ViewItem>,
    ) -> Result<ItemViewDelta, ItemViewError> {
        let payload = self.set_container(open, entries)?;
        let from = self.container_revision;
        let to = advance(&mut self.container_revision)?;
        Ok(ItemViewDelta {
            domain_id: STATE_DOMAIN_OPEN_CONTAINER,
            from,
            to,
            delta_type: DELTA_TYPE_OPEN_CONTAINER_V1,
            payload,
        })
    }

    fn set_inventory(&mut self, inventory: InventoryItems) -> Result<Vec<u8>, ItemViewError> {
        let keys: Vec<ItemKey> = inventory
            .main_backpack
            .iter()
            .chain(&inventory.entries)
            .map(|item| item.key)
            .collect();
        let previous = self.table.keys_of(View::Inventory);
        self.table.replace(View::Inventory, &keys)?;
        let entry = |item: &ViewItem| self.entry(item);
        let view = CharacterInventory {
            main_backpack: inventory.main_backpack.as_ref().map(entry).transpose()?,
            entries: inventory
                .entries
                .iter()
                .map(entry)
                .collect::<Result<_, _>>()?,
            equipment: Vec::new(),
        };
        match encode_character_inventory(&view) {
            Ok(payload) => {
                self.inventory = inventory;
                Ok(payload)
            }
            Err(_) => {
                // An unencodable view is never shown: the previous one stays.
                let _ = self.table.replace(View::Inventory, &previous);
                Err(ItemViewError::Encode)
            }
        }
    }

    fn set_container(
        &mut self,
        open: Option<OpenCorpse>,
        entries: Vec<ViewItem>,
    ) -> Result<Vec<u8>, ItemViewError> {
        let keys: Vec<ItemKey> = open
            .iter()
            .map(|open| open.key)
            .chain(entries.iter().map(|item| item.key))
            .collect();
        let previous = self.table.keys_of(View::Container);
        self.table.replace(View::Container, &keys)?;
        let view = OpenContainer {
            container_handle: open.and_then(|open| self.table.handle(&open.key)),
            entries: entries
                .iter()
                .map(|item| self.entry(item))
                .collect::<Result<_, _>>()?,
        };
        match encode_open_container(&view) {
            Ok(payload) => {
                self.open = open;
                self.open_entries = entries;
                Ok(payload)
            }
            Err(_) => {
                let _ = self.table.replace(View::Container, &previous);
                Err(ItemViewError::Encode)
            }
        }
    }

    fn entry(&self, item: &ViewItem) -> Result<ItemEntry, ItemViewError> {
        Ok(ItemEntry {
            handle: self.table.handle(&item.key).ok_or(ItemViewError::Encode)?,
            item_definition_ref: item.item_definition_ref,
            count: item.count,
            sub_type: item.sub_type,
        })
    }
}

/// Advances one revision before its write (FND-02 §15).
fn advance(revision: &mut u64) -> Result<u64, ItemViewError> {
    *revision = revision.checked_add(1).ok_or(ItemViewError::Exhausted)?;
    Ok(*revision)
}

#[cfg(test)]
#[path = "item_view_tests.rs"]
mod tests;
