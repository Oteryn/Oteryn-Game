use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn pvp_expert_panel(&self) -> oteryn_client_ui::pvp_expert::Panel {
        lock(&self.mailbox).properties.pvp_expert.clone()
    }
}
