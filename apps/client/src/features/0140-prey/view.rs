use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::prey::PreyPanel;

impl PlayLink {
    #[must_use]
    pub fn prey_panel(&self) -> PreyPanel {
        lock(&self.mailbox).prey.clone()
    }
}
