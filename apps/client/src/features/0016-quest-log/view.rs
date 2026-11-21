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



impl PlayLink {
    pub fn open_quest_tracker(&self) {
        self.quest_request(&oteryn_client_ui::quest_log::QuestLogQuery::ListOnly);
        lock(&self.mailbox).quest.tracker_open = true;
    }
}

#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_journal_quest_state(
        &self,
        state: oteryn_client_runtime::quest_log::QuestState,
    ) {
        self.quest_initialize(state);
    }
}
#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_journal_quest_result(&self, result: &oteryn_session::GatedCommandResult) {
        self.quest_result(result);
    }
}
