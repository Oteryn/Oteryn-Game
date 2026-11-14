use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::account_achievements::AchievementsPanel;
impl PlayLink {
    #[must_use]
    pub fn achievements_panel(&self) -> AchievementsPanel {
        lock(&self.mailbox).achievements.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::{open_character, open_window};

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    /// Compare the rendered owner snapshot and enqueue under the same lock.
    pub fn achievements_dispatch_bound(
        &self,
        expected: &AchievementsPanel,
        intent: Option<oteryn_client_ui::account_achievements::Intent>,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed() || mailbox.achievements != *expected {
            return false;
        }
        let panel = &mut mailbox.achievements;
        if !panel.selected || panel.pending.is_some() {
            return false;
        }
        let Some(intent) = intent else {
            panel.open = !panel.open;
            return true;
        };
        let mut next = panel.clone();
        let Some(page) = oteryn_client_ui::account_achievements::request(&mut next, intent) else {
            return false;
        };
        if self.commands.send(crate::combat_input::PlayCommand::AccountAchievements(page)).is_err() {
            return false;
        }
        *panel = next;
        true
    }
}
