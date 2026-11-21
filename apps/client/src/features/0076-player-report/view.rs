use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn player_report_panel(&self) -> oteryn_client_ui::player_report::Panel {
        lock(&self.mailbox).properties.player_report.clone()
    }
}

#[path = "bug.rs"]
pub(in crate::play) mod bug;

#[path = "violation.rs"]
pub(in crate::play) mod violation;
