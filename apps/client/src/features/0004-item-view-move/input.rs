use crate::play::{PlayLink, mailbox::lock};
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn inventory_select(&self, handle: oteryn_session::ItemHandle) -> bool {
        let mut mailbox = lock(&self.mailbox);
        !mailbox.inventory_shutdown && mailbox.inventory.select(handle)
    }

    pub fn inventory_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::inventory::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            let mut mailbox = lock(&self.mailbox);
            if !mailbox.inventory_shutdown {
                mailbox.inventory.open = true;
            }
        }
        true
    }
}
