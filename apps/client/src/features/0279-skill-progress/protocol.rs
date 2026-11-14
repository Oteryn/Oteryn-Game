//! Native admitted query flow for Skill progress.
use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn skill_progress_request(&self) {
        self.skill_progress_request_locked(&mut lock(&self.mailbox));
    }
    pub(in crate::play) fn skill_progress_request_locked(&self, mailbox: &mut crate::play::mailbox::Mailbox) {
        if let Some(payload) =
            oteryn_client_ui::skill_progress::request(&mut mailbox.skill_progress)
            && self
                .commands
                .send(PlayCommand::Feature(FeatureCommand {
                    capability: if mailbox.skill_progress.complete_selected {
                        370
                    } else {
                        279
                    },
                    kind: if mailbox.skill_progress.complete_selected {
                        640
                    } else {
                        495
                    },
                    payload,
                }))
                .is_err()
        {
            mailbox.skill_progress.pending = false;
            mailbox.skill_progress.error = Some(tr!("Session ended"));
        }
    }
    pub fn skill_progress_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::skill_progress::open_key(events) {
            return false;
        }
        self.skill_progress_request();
        #[cfg(windows)]
        crate::play::status_window::open_window(
            self,
            tr!("Skill progress — Ctrl+Alt+Shift+K"),
            |mailbox| &mut mailbox.skill_progress,
        );
        if cfg!(not(windows)) || self.uses_shared_frontend() {
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
    pub(in crate::play) fn skill_display_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.skill_progress.complete_selected = selected;
        mailbox.skill_progress.selected |= selected;
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
#[cfg(all(test, feature = "bevy-shell"))]
impl Outbox {
    pub(crate) fn skills_config_test_progress(&self) {
        let mut mailbox = lock(&self.0);
        mailbox.skill_progress.selected = true;
        mailbox.skill_progress.pending = false;
    }
}
