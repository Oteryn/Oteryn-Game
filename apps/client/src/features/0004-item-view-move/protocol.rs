use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_session::{CharacterInventory, SessionEvent};

impl Outbox {
    pub(in crate::play) fn inventory_initialize(&self, inventory: Option<CharacterInventory>) {
        lock(&self.0).inventory.apply(inventory);
    }

    pub(in crate::play) fn inventory_events(&self, events: &[SessionEvent]) {
        let mut mailbox = lock(&self.0);
        for event in events {
            if let SessionEvent::Inventory(delta) = event {
                mailbox.inventory.apply(Some(delta.value.clone()));
            }
        }
    }

    pub(in crate::play) fn inventory_end(&self) {
        let mut mailbox = lock(&self.0);
        mailbox.inventory = Default::default();
        mailbox.inventory_shutdown = true;
    }
}

impl PlayLink {
    #[must_use]
    pub fn inventory_panel(&self) -> oteryn_client_ui::inventory::InventoryPanel {
        lock(&self.mailbox).inventory.clone()
    }
}
