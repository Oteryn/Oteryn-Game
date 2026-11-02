use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    pub fn loyalty_skills_panel(&self) -> oteryn_client_ui::loyalty_skills::LoyaltySkillsPanel {
        lock(&self.mailbox).loyalty_skills.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
