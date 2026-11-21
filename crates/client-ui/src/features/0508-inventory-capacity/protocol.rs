//! Capability 508: Tibia Global's `Cap` in the inventory panel, from the server's capacity.
use crate::inventory::InventoryPanel;
use oteryn_client_runtime::inventory::InventoryCapacity;

/// Retains the latest capacity the server sent; it replaces the previous one whole.
pub fn apply(panel: &mut InventoryPanel, capacity: InventoryCapacity) {
    panel.capacity = Some(capacity);
}

/// The `Cap` line in whole ounces of free capacity, once the server sent a capacity.
#[must_use]
pub fn cap_text(panel: &InventoryPanel) -> Option<String> {
    panel
        .capacity
        .map(|capacity| lfmt!("Cap: {}", capacity.free_oz()))
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
