//! The House map retains base/content facts while excluding Channel-created movement items.
use super::*;
use crate::map::overlay::AddedItem;

/// A source-qualified state of an immutable bundle placement in this named House.
pub(in crate::gameplay_transport) struct HouseBaseMapEntry {
    pub placement_key: u64,
    pub definition: oteryn_protocol_oteryn::world_map::MapDefinition,
    pub appearance_id: u16,
    pub blocks_projectile: bool,
    pub revision: u64,
}

pub(in crate::gameplay_transport) struct DecoratedFacts<'a, F> {
    pub base: &'a F,
    pub changes: &'a [HouseBaseMapEntry],
}

impl<F: MapFacts> MapFacts for DecoratedFacts<'_, F> {
    fn base_hidden(&self, key: u64) -> bool {
        self.base.base_hidden(key)
    }
    fn base_entry(&self, pos: TilePos, ordinal: u8, id: u32) -> Option<view::EntryFacts> {
        let before = self.base.base_entry(pos, ordinal, id)?;
        let key = view::placement_key(pos, ordinal)?;
        let change = self
            .changes
            .binary_search_by_key(&key, |change| change.placement_key)
            .ok()
            .and_then(|at| self.changes.get(at));
        Some(match change {
            Some(change) => view::EntryFacts {
                definition: change.definition,
                appearance_id: change.appearance_id,
                blocks_projectile: change.blocks_projectile,
                ..before
            },
            None => before,
        })
    }
    fn added_entry(&self, item: &AddedItem) -> Option<view::EntryFacts> {
        self.base.added_entry(item)
    }
    fn house_tile(&self, pos: TilePos) -> bool {
        self.base.house_tile(pos)
    }
    fn base_text(&self, key: u64) -> Option<&str> {
        self.base.base_text(key)
    }
    fn object_revision(&self, key: u64) -> u64 {
        self.changes
            .binary_search_by_key(&key, |change| change.placement_key)
            .ok()
            .and_then(|at| self.changes.get(at))
            .map_or_else(|| self.base.object_revision(key), |change| change.revision)
    }
}

pub(super) struct Facts<'a, F>(pub &'a F);

impl<F: MapFacts> MapFacts for Facts<'_, F> {
    // The default movement_item/movement_items methods are deliberately empty.
    // Those items belong to the Channel callback owner, even at the same coordinates.
    fn base_hidden(&self, key: u64) -> bool {
        self.0.base_hidden(key)
    }
    fn base_entry(&self, pos: TilePos, ordinal: u8, id: u32) -> Option<view::EntryFacts> {
        self.0.base_entry(pos, ordinal, id)
    }
    fn added_entry(&self, item: &AddedItem) -> Option<view::EntryFacts> {
        self.0.added_entry(item)
    }
    fn house_tile(&self, pos: TilePos) -> bool {
        self.0.house_tile(pos)
    }
    fn base_text(&self, key: u64) -> Option<&str> {
        self.0.base_text(key)
    }
    fn object_revision(&self, key: u64) -> u64 {
        self.0.object_revision(key)
    }
}
