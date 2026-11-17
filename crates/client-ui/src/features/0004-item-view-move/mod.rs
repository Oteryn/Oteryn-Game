//! Inventory panel for the live main backpack and its direct contents.
use oteryn_client_runtime::inventory::{CharacterInventory, ItemEntry, ItemHandle, rows};
use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InventoryPanel {
    pub open: bool,
    pub inventory: Option<CharacterInventory>,
    selected: Option<ItemHandle>,
}

impl InventoryPanel {
    pub fn apply(&mut self, inventory: Option<CharacterInventory>) {
        self.inventory = inventory;
        if self.selected_item().is_none() {
            self.selected = None;
        }
    }

    #[must_use]
    pub fn rows(&self) -> Vec<ItemEntry> {
        self.inventory.as_ref().map(rows).unwrap_or_default()
    }

    pub fn select(&mut self, handle: ItemHandle) -> bool {
        if self.rows().iter().any(|item| item.handle == handle) {
            self.selected = Some(handle);
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn selected_item(&self) -> Option<ItemEntry> {
        self.rows()
            .into_iter()
            .find(|item| Some(item.handle) == self.selected)
    }

    #[must_use]
    pub fn text(&self) -> String {
        if self.inventory.is_none() {
            return tr!("Inventory unavailable for this session").into();
        }
        if self.rows().is_empty() {
            return tr!("Backpack empty").into();
        }
        match self.selected_item() {
            Some(item) => lfmt!(
                "Selected: {} ×{} (handle {}, subtype {})",
                item.item_definition_ref,
                item.count,
                item.handle,
                item.sub_type
            ),
            None => tr!("Select a backpack or item").into(),
        }
    }
}

/// Ctrl+I opens the panel; key repeats and other modifiers do not consume play input.
#[must_use]
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 12 && modifiers.contains(Modifier::Control) && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)))
}

#[cfg(test)]
mod tests;

#[path = "state.rs"]
pub mod movement;
