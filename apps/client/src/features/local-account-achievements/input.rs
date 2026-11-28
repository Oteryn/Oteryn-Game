use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::account_achievements::{
    AchievementFilter, AchievementSort, AchievementsPanel, Intent,
};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

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
        let panel = &mut mailbox.achievements;
        if let Some(page) = oteryn_client_ui::account_achievements::request(panel, intent) {
            send_page(panel, &self.commands, page);
        }
    }

    pub fn achievements_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::account_achievements::open_key(events) {
            return false;
        }
        self.achievements_request(Intent::Refresh);
        lock(&self.mailbox).achievements.open = true;
        true
    }
}

pub(super) fn send_page(
    panel: &mut AchievementsPanel,
    commands: &UnboundedSender<PlayCommand>,
    page: u32,
) {
    let command = match panel.completion {
        Some(mode) => PlayCommand::AccountAchievementsFiltered(page, mode),
        None => PlayCommand::AccountAchievements(page),
    };
    if commands.send(command).is_err() {
        panel.pending = None;
        panel.catalog.collecting = false;
        panel.error = Some(tr!("Session ended"));
    }
}
