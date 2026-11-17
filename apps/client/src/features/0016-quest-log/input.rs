use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::quest_log::{QuestLogQuery, QuestSort, request};
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn quest_list_options(&self, search: &str, sort: QuestSort) {
        lock(&self.mailbox).quest.set_list_options(search, sort);
    }

    /// Feature controls enter the same chronological session command channel as movement.
    pub fn quest_request(&self, query: &QuestLogQuery) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.quest_pending {
            return;
        }
        if let Ok(payload) = request(&mut mailbox.quest, query) {
            mailbox.quest_pending = self.commands.send(PlayCommand::QuestLog(payload)).is_ok();
        }
    }

    /// Opens the feature panel on Ctrl+J; returns whether the feature consumed the event.
    pub fn quest_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::quest_log::tracker_key(events) {
            self.open_quest_tracker();
            return true;
        }
        if !oteryn_client_ui::quest_log::open_key(events) {
            return false;
        }
        if !self.quest_panel().open {
            self.quest_request(&QuestLogQuery::List);
        }
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).quest.open = true;
        }
        true
    }
}
