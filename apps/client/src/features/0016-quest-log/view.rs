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
            mailbox.quest.tracker_selection = None;
        }
    }
    /// Mission selection is accepted only against the currently confirmed quest line.
    pub fn quest_select_mission(&self, quest_index: u32, mission_index: u32) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.quest;
        if !panel.state.selected
            || !panel.journal_line().is_some_and(|line| {
                line.quest_index == quest_index
                    && line
                        .missions
                        .iter()
                        .any(|m| m.mission_index == mission_index)
            })
        {
            return false;
        }
        panel.mission = Some(mission_index);
        true
    }

    #[cfg(bevy_shell)]
    pub(crate) fn update_quest_snapshot(&self, snapshot: &mut QuestPanel) -> bool {
        let mailbox = lock(&self.mailbox);
        if mailbox.quest == *snapshot {
            return false;
        }
        snapshot.clone_from(&mailbox.quest);
        true
    }

    #[cfg(bevy_shell)]
    pub(crate) fn quest_sort_selection(&self) -> usize {
        lock(&self.mailbox).quest.list_sort.selection()
    }

    /// A consistent projection for the native feature window and recorded-frame tests.
    #[must_use]
    pub fn quest_panel(&self) -> QuestPanel {
        let mut panel = lock(&self.mailbox).quest.clone();
        if let Some(line) = panel.tracker_line() {
            panel.state.log.view = Some(oteryn_client_ui::quest_log::QuestLogView::Line(
                line.clone(),
            ));
        }
        panel
    }
}

impl PlayLink {
    pub fn quest_tracker_visible(&self) -> bool {
        lock(&self.mailbox).quest.tracker_open
    }
    pub fn quest_close_tracker(&self) {
        lock(&self.mailbox).quest.tracker_open = false;
    }
    pub fn quest_open_tracked_mission(&self, quest: u32, mission: u32) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.quest;
        if panel.tracker_open
            && panel.state.log.tracked.iter().any(|l| {
                l.quest_index == quest && l.missions.iter().any(|m| m.mission_index == mission)
            })
        {
            panel.tracker_selection = Some(quest);
            panel.mission = Some(mission);
            panel.open = true;
        }
    }
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
