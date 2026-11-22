//! Coalesced native actor identity follows the current session across scope changes.
use super::{Mailbox, Outbox, lock};

impl Outbox {
    pub(in crate::play) fn own_identity(&self, identity: Option<[u8; 16]>) {
        lock(&self.0).own_identity(identity);
    }
}

impl Mailbox {
    pub(in crate::play) fn own_identity(&mut self, identity: Option<[u8; 16]>) {
        if let Some(identity) = identity
            && self.own_actor_identity != Some(identity)
        {
            self.own_actor_identity = Some(identity);
            self.pushed.own_identity = Some(identity);
        }
    }
}
