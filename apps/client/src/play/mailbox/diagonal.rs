//! Negotiated eight-way input reuses the bounded movement command mailbox.
use super::{Outbox, PlayLink, lock};
use oteryn_client_domain::movement::StepDir;
use oteryn_input_actions::NormalizedInputEvent;
impl Outbox {
    pub(in crate::play) fn diagonal_initialize(&self, selected: bool) {
        lock(&self.0).diagonal_selected = selected;
    }
}
impl PlayLink {
    pub fn diagonal_input(&self, events: &[NormalizedInputEvent]) -> Option<StepDir> {
        let mailbox = lock(&self.mailbox);
        if mailbox.world_input_blocked() {
            return None;
        }
        oteryn_client_ui::diagonal_movement::step(events, mailbox.diagonal_selected)
    }
}
