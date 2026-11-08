use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn analyser_initialize(&self, selected: bool) {
        lock(&self.0).analyser = oteryn_client_ui::analyser::Panel {
            selected,
            ..Default::default()
        };
    }
}

pub(in crate::play) fn reduce(
    mailbox: &mut crate::play::mailbox::Mailbox,
    events: &[oteryn_session::SessionEvent],
) {
    use oteryn_session::SessionEvent;
    for event in events {
        match event {
            SessionEvent::AnalyserFacts(delta) => {
                oteryn_client_ui::analyser::protocol::apply(&mut mailbox.analyser, &delta.value)
            }
            SessionEvent::GatedSnapshotReplaced(delta) if delta.value.domain_id == 15 => {
                // oteryn-choice: recovery cannot replay facts. Start a fresh measurement interval.
                oteryn_client_ui::analyser::protocol::reset(&mut mailbox.analyser);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "tests/e2e_n07c.rs"]
mod e2e_n07c;
