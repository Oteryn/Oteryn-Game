//! Ten visible equipment slots, projected only from the authoritative inventory snapshot.
use crate::inventory::movement::DragItem;
pub use oteryn_client_runtime::equipment::{
    CharacterInventory, EquipmentSlot, EquippedItem, ItemEntry, ItemMoveDestination,
    WorldTilePosition,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub label: &'static str,
    pub destination: ItemMoveDestination,
    pub item: Option<ItemEntry>,
    pub available: bool,
}
impl Slot {
    #[must_use]
    pub fn text(self) -> String {
        if !self.available {
            return format!("{}: unavailable", self.label);
        }
        match self.item {
            Some(item) => format!(
                "{}: {} ×{}",
                self.label, item.item_definition_ref, item.count
            ),
            None => format!("{}: empty", self.label),
        }
    }
    #[must_use]
    pub fn drag_item(self) -> Option<DragItem> {
        if !self.available || self.destination == ItemMoveDestination::MainBackpack {
            return None;
        }
        self.item.map(|item| DragItem {
            equipped: true,
            handle: item.handle,
            count: item.count.get(),
        })
    }
}

#[must_use]
pub fn rows(inventory: Option<&CharacterInventory>, selected: bool) -> Vec<Slot> {
    use EquipmentSlot as E;
    [
        ("Head", Some(E::Head)),
        ("Neck", Some(E::Necklace)),
        ("Backpack", None),
        ("Armor", Some(E::Armor)),
        ("Right", Some(E::RightHand)),
        ("Left", Some(E::LeftHand)),
        ("Legs", Some(E::Legs)),
        ("Feet", Some(E::Feet)),
        ("Ring", Some(E::Ring)),
        ("Ammo", Some(E::Ammo)),
    ]
    .into_iter()
    .map(|(label, slot)| Slot {
        label,
        destination: slot.map_or(
            ItemMoveDestination::MainBackpack,
            ItemMoveDestination::Equipment,
        ),
        item: inventory.and_then(|inventory| match slot {
            Some(slot) => inventory
                .equipment
                .iter()
                .find(|entry| entry.slot == slot)
                .map(|entry| entry.item),
            None => inventory.main_backpack,
        }),
        available: selected && inventory.is_some(),
    })
    .collect()
}

#[cfg(test)]
#[path = "tests/panel.rs"]
mod tests;
