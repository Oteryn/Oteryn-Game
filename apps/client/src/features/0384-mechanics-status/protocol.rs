//! Native admitted query flow for Character effects.
use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn mechanics_status_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(payload) =
            oteryn_client_ui::mechanics_status::request(&mut mailbox.mechanics_status)
            && self
                .commands
                .send(PlayCommand::Feature(FeatureCommand {
                    capability: 384,
                    kind: 669,
                    payload,
                }))
                .is_err()
        {
            mailbox.mechanics_status.pending = false;
            mailbox.mechanics_status.error = Some(tr!("Session ended"));
        }
    }
    pub fn mechanics_status_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::mechanics_status::open_key(events) {
            return false;
        }
        self.mechanics_status_request();
        #[cfg(windows)]
        crate::play::status_window::open_window(
            self,
            tr!("Character effects — Ctrl+Alt+Shift+U"),
            |mailbox| &mut mailbox.mechanics_status,
        );
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).mechanics_status.open = true;
        }
        true
    }
    #[cfg(test)]
    pub(in crate::play) fn mechanics_status_panel(
        &self,
    ) -> oteryn_client_ui::mechanics_status::MechanicsStatusPanel {
        lock(&self.mailbox).mechanics_status.clone()
    }
}
impl Outbox {
    pub(in crate::play) fn mechanics_status_initialize(&self, selected: bool) {
        lock(&self.0).mechanics_status.selected = selected;
    }
    pub(in crate::play) fn mechanics_status_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let accepted = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        );
        lock(&self.0)
            .mechanics_status
            .finish(accepted, &result.payload);
    }
}
