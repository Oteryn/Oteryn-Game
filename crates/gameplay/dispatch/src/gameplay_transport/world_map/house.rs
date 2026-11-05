//! The House map retains base/content facts while excluding Channel-created movement items.
use super::*;
use crate::map::overlay::AddedItem;

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
