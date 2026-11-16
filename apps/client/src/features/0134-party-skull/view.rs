use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn party_skull_panel(&self) -> oteryn_client_ui::party_skull::Panel {
        lock(&self.mailbox).properties.party_skull.clone()
    }
}

#[path = "pvp_status.rs"]
pub(in crate::play) mod pvp_status;
