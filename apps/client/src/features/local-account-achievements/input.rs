use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::account_achievements::{AchievementFilter, AchievementSort, Intent};
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn achievements_browse(
        &self,
        search: &str,
        filter: AchievementFilter,
        sort: AchievementSort,
    ) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.achievements.selected {
            mailbox
                .achievements
                .catalog
                .set_options(search, filter, sort);
        }
    }

    pub fn achievements_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(payload) =
            oteryn_client_ui::account_achievements::request(&mut mailbox.achievements, intent)
            && self
                .commands
                .send(PlayCommand::AccountAchievements(payload))
                .is_err()
        {
            mailbox.achievements.pending = None;
            mailbox.achievements.error = Some(tr!("Session ended"));
        }
    }

    pub fn achievements_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::account_achievements::open_key(events) {
            return false;
        }
        self.achievements_request(Intent::Refresh);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).achievements.open = true;
        }
        true
    }
}
