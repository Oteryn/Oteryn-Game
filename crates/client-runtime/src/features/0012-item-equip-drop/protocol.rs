//! Equipment uses the existing capability-12 command-9 destination and inventory stream.
pub use oteryn_session::item_movement::{EquipmentSlot, EquippedItem, WorldTilePosition};
pub use oteryn_session::{CharacterInventory, ItemEntry, ItemMoveDestination};

#[must_use]
pub fn available(destination: ItemMoveDestination, equip_drop: bool) -> bool {
    !matches!(destination, ItemMoveDestination::Equipment(_)) || equip_drop
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equipment_requires_negotiated_capability() {
        for slot in EquipmentSlot::ALL {
            assert!(!available(ItemMoveDestination::Equipment(slot), false));
            assert!(available(ItemMoveDestination::Equipment(slot), true));
        }
        assert!(available(ItemMoveDestination::MainBackpack, false));
    }
}
