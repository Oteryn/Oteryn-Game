use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::quest_log::QuestPanel;

impl PlayLink {
    #[must_use]
    pub fn quest_window_open(&self) -> bool {
        let mailbox = lock(&self.mailbox);
        mailbox.quest.open || mailbox.quest.tracker_open
    }

    /// Presentation visibility only; confirmed quest/tracker state is retained.
    pub fn quest_set_open(&self, open: bool) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.quest.open = open;
        if !open {
            mailbox.quest.tracker_open = false;
        }
    }
    /// Mission selection is accepted only against the currently confirmed quest line.
    pub fn quest_select_mission(&self, quest_index: u32, mission_index: u32) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.quest;
        if !panel.state.selected
            || !matches!(&panel.state.log.view,
            Some(oteryn_client_ui::quest_log::QuestLogView::Line(line))
                if line.quest_index == quest_index && line.missions.iter().any(|mission| mission.mission_index == mission_index))
        {
            return false;
        }
        panel.mission = Some(mission_index);
        true
    }

    /// A consistent projection for the native feature window and recorded-frame tests.
    #[must_use]
    pub fn quest_panel(&self) -> QuestPanel {
        lock(&self.mailbox).quest.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(windows)]
#[path = "native/tracker.rs"]
mod tracker;

impl PlayLink {
    pub fn open_quest_tracker(&self) {
        self.quest_request(&oteryn_client_ui::quest_log::QuestLogQuery::ListOnly);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            tracker::open(
                std::sync::Arc::downgrade(&self.mailbox),
                self.commands.clone(),
            );
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).quest.tracker_open = true;
        }
    }
}

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_journal_quest_state(
        &self,
        state: oteryn_client_runtime::quest_log::QuestState,
    ) {
        self.quest_initialize(state);
    }
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_journal_quest_result(&self, result: &oteryn_session::GatedCommandResult) {
        self.quest_result(result);
    }
}
