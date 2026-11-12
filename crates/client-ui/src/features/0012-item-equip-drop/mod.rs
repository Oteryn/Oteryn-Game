//! Capability-12 equipment projection and command-9 input integration.
pub mod protocol;
mod state;
pub use state::{
    CharacterInventory, EquipmentSlot, EquippedItem, ItemEntry, ItemMoveDestination, Slot,
    WorldTilePosition, rows,
};
