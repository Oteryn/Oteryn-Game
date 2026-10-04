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
//!
//! Capability 4 stays `offered: false` until ITEM-MOVE-1, so production selects it never and only
//! the tests negotiate it.

#![cfg_attr(not(test), allow(dead_code))]

#[path = "../interaction/corpse_open.rs"]
pub(crate) mod corpse_open;

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
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

/// The server-internal identity of one viewed item. Never on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ItemKey {
    /// A domain-1 object (a corpse or a ground item) by its channel-wide `EntityIdentity`.
    Entity([u8; ENTITY_IDENTITY_BYTES]),
    /// A live item instance: the main backpack, its entries and a corpse's entries.
    Instance([u8; 16]),
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
}

/// The session's handle table (§4.1).
#[derive(Debug, Default)]
pub(crate) struct ItemHandleTable {
    last: u64,
    views: [BTreeSet<ItemKey>; 4],
    /// Capability 14 is selected: the bound is `ITEMV0-RL-03-CONTAINER-TREE`.
    container_tree: bool,
    by_key: BTreeMap<ItemKey, ItemHandle>,
    by_handle: BTreeMap<ItemHandle, ItemKey>,
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

    /// The bound of this session's live handles.
    pub(crate) const fn limit(&self) -> usize {
        if self.container_tree {
            MAX_LIVE_ITEM_HANDLES_CONTAINER_TREE
        } else {
            MAX_LIVE_ITEM_HANDLES
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
        self.by_handle.get(&handle).copied()
    }

    pub(crate) fn handle(&self, key: &ItemKey) -> Option<ItemHandle> {
        self.by_key.get(key).copied()
    }

    /// Makes `keys` the content of `view`: an item new to every view gets the next handle, in the
    /// given order, and one that left every view is dropped. Over the bound nothing changes.
    pub(crate) fn replace(&mut self, view: View, keys: &[ItemKey]) -> Result<(), ItemViewError> {
        let next: BTreeSet<ItemKey> = keys.iter().copied().collect();
        let others = |key: &ItemKey| {
            self.views
                .iter()
                .enumerate()
                .any(|(index, set)| index != view as usize && set.contains(key))
        };
        let staying = self.by_key.keys().filter(|key| others(key)).count();
        let live = staying + next.iter().filter(|key| !others(key)).count();
        if live > self.limit() {
            return Err(ItemViewError::LimitExceeded);
        }
        let fresh = next
            .iter()
            .filter(|key| !self.by_key.contains_key(key))
            .count();
        self.last
            .checked_add(fresh as u64)
            .ok_or(ItemViewError::Exhausted)?;
        let dropped: Vec<ItemKey> = self.views[view as usize]
            .iter()
            .filter(|key| !next.contains(key) && !others(key))
            .copied()
            .collect();
        for key in dropped {
            if let Some(handle) = self.by_key.remove(&key) {
                self.by_handle.remove(&handle);
            }
        }
        for key in keys {
            if self.by_key.contains_key(key) {
                continue;
            }
            self.last += 1;
            let handle = ItemHandle::new(self.last).ok_or(ItemViewError::Exhausted)?;
            self.by_key.insert(*key, handle);
            self.by_handle.insert(handle, *key);
        }
        self.views[view as usize] = next;
        Ok(())
    }
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
        }
    }

    /// Capability 14 is selected: the handle bound becomes `ITEMV0-RL-03-CONTAINER-TREE`.
    pub(crate) fn with_container_tree(mut self) -> Self {
        self.table = self.table.with_container_tree();
        self
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
        let previous = self.table.views[View::Tree as usize]
            .iter()
            .copied()
            .collect::<Vec<_>>();
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
        let previous = self.table.views[View::Inventory as usize]
            .iter()
            .copied()
            .collect::<Vec<_>>();
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
        let previous = self.table.views[View::Container as usize]
            .iter()
            .copied()
            .collect::<Vec<_>>();
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
