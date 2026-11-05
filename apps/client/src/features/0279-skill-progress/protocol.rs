//! Native admitted query flow for Skill progress.
use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn skill_progress_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(payload) =
            oteryn_client_ui::skill_progress::request(&mut mailbox.skill_progress)
            && self
                .commands
                .send(PlayCommand::Feature(FeatureCommand {
                    capability: 279,
                    kind: 495,
                    payload,
                }))
                .is_err()
        {
            mailbox.skill_progress.pending = false;
            mailbox.skill_progress.error = Some("Session ended");
        }
    }
    pub fn skill_progress_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::skill_progress::open_key(events) {
            return false;
        }
        self.skill_progress_request();
        #[cfg(windows)]
        crate::play::status_window::open_window(self, "Skill progress — Ctrl+Alt+Shift+K", |mailbox| {
            &mut mailbox.skill_progress
        });
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).skill_progress.open = true;
        }
        true
    }
    pub fn skill_progress_panel(&self) -> oteryn_client_ui::skill_progress::SkillProgressPanel {
        lock(&self.mailbox).skill_progress.clone()
    }
}
impl Outbox {
    pub(in crate::play) fn skill_progress_initialize(&self, selected: bool) {
        lock(&self.0).skill_progress.selected = selected;
    }
    pub(in crate::play) fn skill_progress_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let accepted = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        );
        lock(&self.0)
            .skill_progress
            .finish(accepted, &result.payload);
    }
}
