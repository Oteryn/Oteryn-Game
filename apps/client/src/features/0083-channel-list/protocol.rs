use crate::play::{Outbox, mailbox::{Mailbox, lock}};
impl Outbox {
    pub(in crate::play) fn channel_list_initialize(&self, payload: Option<Vec<u8>>) {
        lock(&self.0).channel_list_initialize(payload.as_deref());
    }
}

impl Mailbox {
    pub(in crate::play) fn channel_list_events(&mut self, events: &[oteryn_session::SessionEvent]) {
        for event in events {
            if let oteryn_session::SessionEvent::GatedSnapshotReplaced(delta) = event
                && delta.value.domain_id == oteryn_session::channel_list::STATE_DOMAIN_CHANNEL_LIST
            {
                self.channel_list_initialize(Some(&delta.value.payload));
            }
        }
    }
    fn channel_list_initialize(&mut self, payload: Option<&[u8]>) {
        // oteryn-choice: publish clears only after a channel was displayed; unselected startup is silent.
        let channel =
            payload.and_then(|bytes| oteryn_client_runtime::channel_list::decode(bytes).ok());
        if self.served_channel != channel {
            self.served_channel = channel;
            self.pushed.served_channel = Some(channel);
        }
        self.party.own_channel = channel.map(|channel| channel.channel_id).or(self.channel_switch.current);
    }
}
