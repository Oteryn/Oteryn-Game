use crate::play::{
    Outbox, PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_session::{ActorVitals, SessionEvent};

pub(in crate::play) fn vitals_events(mailbox: &mut Mailbox, events: &[SessionEvent]) {
    for event in events {
        if let SessionEvent::ActorVitals(delta) = event {
            mailbox.vitals = Some(delta.value);
        }
    }
}

impl Outbox {
    pub(in crate::play) fn vitals(&self, vitals: Option<ActorVitals>) {
        if let Some(vitals) = vitals {
            lock(&self.0).vitals = Some(vitals);
        }
    }
}

impl PlayLink {
    #[must_use]
    pub fn actor_vitals(&self) -> Option<ActorVitals> {
        lock(&self.mailbox).vitals
    }
}
