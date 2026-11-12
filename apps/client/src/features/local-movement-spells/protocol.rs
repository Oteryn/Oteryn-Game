use crate::play::{Outbox, Stepper, mailbox::lock};

impl Outbox {
    pub(in crate::play) fn chat_with_movement_initialize(&self, session: &impl Stepper) {
        self.chat_public_initialize(session.chat_public_selected(), session.chat_state());
        self.movement_spells_initialize(session.feature_selected(7));
    }

    pub(in crate::play) fn movement_spells_initialize(&self, selected: bool) {
        lock(&self.0).chat_public.speech_selected = selected;
    }
}
