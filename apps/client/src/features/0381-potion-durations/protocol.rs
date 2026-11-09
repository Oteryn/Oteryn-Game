//! Native admitted query flow for Potion durations.
use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn potion_durations_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(payload) =
            oteryn_client_ui::potion_durations::request(&mut mailbox.potion_durations)
            && self
                .commands
                .send(PlayCommand::Feature(FeatureCommand {
                    capability: 381,
                    kind: 662,
                    payload,
                }))
                .is_err()
        {
            mailbox.potion_durations.pending = false;
            mailbox.potion_durations.error = Some(tr!("Session ended"));
        }
    }
    pub fn potion_durations_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::potion_durations::open_key(events) {
            return false;
        }
        self.potion_durations_request();
        #[cfg(windows)]
        crate::play::status_window::open_window(
            self,
            tr!("Potion durations — Ctrl+Alt+Shift+T"),
            |mailbox| &mut mailbox.potion_durations,
        );
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).potion_durations.open = true;
        }
        true
    }
    #[cfg(test)]
    pub(in crate::play) fn potion_durations_panel(
        &self,
    ) -> oteryn_client_ui::potion_durations::PotionDurationsPanel {
        lock(&self.mailbox).potion_durations.clone()
    }
}
impl Outbox {
    pub(in crate::play) fn potion_durations_initialize(&self, selected: bool) {
        lock(&self.0).potion_durations.selected = selected;
    }
    pub(in crate::play) fn potion_durations_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let accepted = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        );
        lock(&self.0)
            .potion_durations
            .finish(accepted, &result.payload);
    }
}
