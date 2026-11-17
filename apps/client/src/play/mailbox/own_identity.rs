//! Coalesced native actor identity follows the current session across scope changes.
use super::{Outbox, lock};

impl Outbox {
    pub(in crate::play) fn own_identity(&self, identity: Option<[u8; 16]>) {
        if let Some(identity) = identity {
            let mut mailbox = lock(&self.0);
            if mailbox.own_actor_identity != Some(identity) {
                mailbox.own_actor_identity = Some(identity);
                mailbox.pushed.own_identity = Some(identity);
            }
        }
    }
}
