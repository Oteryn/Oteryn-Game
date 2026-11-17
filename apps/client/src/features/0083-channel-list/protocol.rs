use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn channel_list_events(&self, events: &[oteryn_session::SessionEvent]) {
        for event in events {
            if let oteryn_session::SessionEvent::GatedSnapshotReplaced(delta) = event
                && delta.value.domain_id == 48
            {
                self.channel_list_initialize(Some(delta.value.payload.clone()));
            }
        }
    }
    pub(in crate::play) fn channel_list_initialize(&self, payload: Option<Vec<u8>>) {
        // oteryn-choice: publish clears only after a channel was displayed; unselected startup is silent.
        let channel =
            payload.and_then(|bytes| oteryn_client_runtime::channel_list::decode(&bytes).ok());
        let mut mailbox = lock(&self.0);
        if mailbox.served_channel != channel {
            mailbox.served_channel = channel;
            mailbox.pushed.served_channel = Some(channel);
        }
    }
}
