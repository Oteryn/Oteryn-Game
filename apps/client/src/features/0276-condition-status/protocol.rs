//! Native admitted query flow for Conditions.
use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn condition_status_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(payload) =
            oteryn_client_ui::condition_status::request(&mut mailbox.condition_status)
            && self
                .commands
                .send(PlayCommand::Feature(FeatureCommand {
                    capability: 276,
                    kind: 492,
                    payload,
                }))
                .is_err()
        {
            mailbox.condition_status.pending = false;
            mailbox.condition_status.error = Some(tr!("Session ended"));
        }
    }
    pub fn condition_status_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::condition_status::open_key(events) {
            return false;
        }
        self.condition_status_request();
        #[cfg(windows)]
        crate::play::status_window::open_window(
            self,
            tr!("Conditions — Ctrl+Alt+Shift+C"),
            |mailbox| &mut mailbox.condition_status,
        );
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).condition_status.open = true;
        }
        true
    }
    pub fn condition_status_panel(
        &self,
    ) -> oteryn_client_ui::condition_status::ConditionStatusPanel {
        lock(&self.mailbox).condition_status.clone()
    }
}
impl Outbox {
    pub(in crate::play) fn condition_status_initialize(&self, selected: bool) {
        lock(&self.0).condition_status.selected = selected;
    }
    pub(in crate::play) fn condition_status_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let accepted = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        );
        lock(&self.0)
            .condition_status
            .finish(accepted, &result.payload);
    }
}

#[cfg(feature = "bevy-shell")]
pub(crate) mod details;
