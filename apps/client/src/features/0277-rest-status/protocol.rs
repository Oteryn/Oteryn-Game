//! Native admitted query flow for Rest and food.
use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn rest_status_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(payload) = oteryn_client_ui::rest_status::request(&mut mailbox.rest_status)
            && self
                .commands
                .send(PlayCommand::Feature(FeatureCommand {
                    capability: 277,
                    kind: 493,
                    payload,
                }))
                .is_err()
        {
            mailbox.rest_status.pending = false;
            mailbox.rest_status.error = Some(tr!("Session ended"));
        }
    }
    pub fn rest_status_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::rest_status::open_key(events) {
            return false;
        }
        self.rest_status_request();
        #[cfg(windows)]
        crate::play::status_window::open_window(
            self,
            tr!("Rest and food — Ctrl+Alt+Shift+R"),
            |mailbox| &mut mailbox.rest_status,
        );
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).rest_status.open = true;
        }
        true
    }
    pub fn rest_status_panel(&self) -> oteryn_client_ui::rest_status::RestStatusPanel {
        lock(&self.mailbox).rest_status.clone()
    }
}
impl Outbox {
    pub(in crate::play) fn rest_status_initialize(&self, selected: bool) {
        lock(&self.0).rest_status.selected = selected;
    }
    pub(in crate::play) fn rest_status_result(&self, result: &oteryn_session::GatedCommandResult) {
        let accepted = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        );
        lock(&self.0).rest_status.finish(accepted, &result.payload);
    }
}
