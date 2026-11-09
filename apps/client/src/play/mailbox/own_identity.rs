//! Coalesced native actor identity follows the current session across scope changes.
use super::{Outbox, lock};
use oteryn_session::WorldSpatialEntity;

impl Outbox {
    pub(in crate::play) fn entities(&self, entities: Vec<WorldSpatialEntity>) {
        let mut mailbox = lock(&self.0);
        oteryn_client_ui::player_typing::protocol::retain_visible(
            &mut mailbox.player_typing,
            &entities,
        );
        mailbox.pushed.entities = Some(entities);
    }

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
