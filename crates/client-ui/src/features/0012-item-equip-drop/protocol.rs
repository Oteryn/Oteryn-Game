//! Inventory deltas carry capability-12 equipment; movement reuses command 9.
pub fn project(inventory: Option<&super::CharacterInventory>, selected: bool) -> Vec<super::Slot> {
    super::rows(inventory, selected)
}
