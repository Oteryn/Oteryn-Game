//! Test access to the existing initialization and server reply paths.
use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(crate) fn bevy_store_initialize(&self) {
        self.store_initialize(true, false);
        lock(&self.0).discovery.history.selected = true;
    }
    pub(crate) fn bevy_store_response(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.store_result(response);
    }
    pub(crate) fn bevy_store_history_response(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.discovery_result(285, response);
    }
    pub(crate) fn bevy_store_gift_response(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.coin_gift_result(response);
    }
    pub(crate) fn bevy_store_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

impl Outbox {
    pub(crate) fn bevy_store_copy_view(&self, snapshot: &super::StoreSnapshot) {
        let mut mailbox = lock(&self.0);
        mailbox.store = snapshot.panel.clone();
        mailbox.discovery.history = snapshot.history.clone();
    }
}
