use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::quest_log::{QuestLogQuery, QuestSort, request};
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn quest_list_options(&self, search: &str, sort: QuestSort) {
        lock(&self.mailbox).quest.set_list_options(search, sort);
    }

    pub fn quest_toggle_hidden_filter(&self) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.quest.show_hidden = !mailbox.quest.show_hidden;
    }

    pub fn quest_toggle_mark(&self, pin: bool) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.quest;
        let Some(line) = panel.journal_line() else {
            return false;
        };
        let id = line.quest_index;
        if !panel.state.selected
            || !panel.state.list.iter().any(|q| q.quest_index == id)
            || (!panel.show_hidden && panel.hidden.contains(&id))
        {
            return false;
        }
        let marks = if pin {
            &mut panel.pinned
        } else {
            &mut panel.hidden
        };
        if !marks.remove(&id) {
            marks.insert(id);
        }
        true
    }

    /// Feature controls enter the same chronological session command channel as movement.
    pub fn quest_request(&self, query: &QuestLogQuery) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.quest_pending {
            return;
        }
        if let Ok(payload) = request(&mut mailbox.quest, query) {
            if matches!(query, QuestLogQuery::Quest { .. } | QuestLogQuery::List) {
                mailbox.quest.tracker_selection = None;
            }
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
        lock(&self.mailbox).quest.open = true;
        true
    }
}
