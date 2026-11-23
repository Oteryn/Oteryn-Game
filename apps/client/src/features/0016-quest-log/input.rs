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

    pub fn quest_toggle_mark(&self, displayed: Option<u32>, pin: bool) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.quest;
        let Some(line) = panel.journal_line() else {
            return false;
        };
        let id = line.quest_index;
        if displayed != Some(id)
            || !panel.state.selected
            || !panel.state.list.iter().any(|q| q.quest_index == id)
            || (panel.show_hidden != panel.hidden.contains(&id))
        {
            return false;
        }
        if pin {
            if let Some(pos) = panel.pinned.iter().position(|p| *p == id) {
                panel.pinned.remove(pos);
            } else {
                panel.pinned.insert(0, id);
            }
        } else if !panel.hidden.remove(&id) {
            panel.hidden.insert(id);
        }
        true
    }

    /// Feature controls enter the same chronological session command channel as movement.
    pub fn quest_request(&self, query: &QuestLogQuery) {
        self.enqueue_quest_request(&mut lock(&self.mailbox), query);
    }

    pub fn quest_clean_tracker(&self, completed_only: bool) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mailbox.quest;
        if !panel.open || !panel.state.selected {
            return false;
        }
        let tracked = &panel.state.log.tracked;
        let quest_indexes: Vec<_> = tracked
            .iter()
            .filter(|line| completed_only && !line.completed)
            .map(|line| line.quest_index)
            .collect();
        if quest_indexes.len() == tracked.len() {
            return false;
        }
        self.enqueue_quest_request(&mut mailbox, &QuestLogQuery::Track { quest_indexes })
    }

    fn enqueue_quest_request(
        &self,
        mailbox: &mut crate::play::mailbox::Mailbox,
        query: &QuestLogQuery,
    ) -> bool {
        if mailbox.quest_pending {
            return false;
        }
        if let Ok(payload) = request(&mut mailbox.quest, query) {
            if matches!(query, QuestLogQuery::Quest { .. } | QuestLogQuery::List) {
                mailbox.quest.tracker_selection = None;
            }
            mailbox.quest_pending = self.commands.send(PlayCommand::QuestLog(payload)).is_ok();
            if !mailbox.quest_pending {
                mailbox.quest.pending_query = None;
                mailbox.quest.state.refusal =
                    Some(oteryn_client_ui::quest_log::QuestRefusal::Rejected);
            }
            return mailbox.quest_pending;
        }
        false
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
