use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn charm_progression_panel(&self) -> oteryn_client_ui::charm_progression::Panel {
        lock(&self.mailbox).properties.charm_progression.clone()
    }
}

