use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn charm_progression_panel(&self) -> oteryn_client_ui::charm_progression::Panel {
        let mut panel = lock(&self.mailbox).properties.charm_progression.clone();
        panel.capture_pending = false;
        panel.capture_stages = None;
        panel.discard_capture = false;
        panel
    }
}
