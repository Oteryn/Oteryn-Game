use crate::play::{PlayLink, mailbox::lock};
#[path = "layout.rs"]
mod layout;
#[cfg(feature = "bevy-shell")]
pub use layout::SkillsNumbers;
pub use layout::{SkillsLayout, SkillsLayoutState, SkillsWidget};

impl PlayLink {
    pub fn skills_widget(&self) -> SkillsWidget {
        let mailbox = lock(&self.mailbox);
        mailbox
            .skills_layout
            .preferences
            .project(&mailbox.skill_progress, &mailbox.loyalty_skills)
    }

    pub fn sync_skills_layout(&self, saved: &mut SkillsLayout, saved_edit: u64) -> Option<u64> {
        let mut mailbox = lock(&self.mailbox);
        let layout = &mut mailbox.skills_layout;
        if layout.dirty && layout.edit > saved_edit {
            saved.clone_from(&layout.preferences);
            layout.dirty = false;
            Some(layout.edit)
        } else {
            layout.dirty = false;
            layout.preferences.clone_from(saved);
            None
        }
    }

    pub fn loyalty_skills_panel(&self) -> oteryn_client_ui::loyalty_skills::LoyaltySkillsPanel {
        lock(&self.mailbox).loyalty_skills.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
