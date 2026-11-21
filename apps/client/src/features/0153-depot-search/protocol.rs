use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn depot_search_initialize(&self, selected: bool) {
        lock(&self.0).depot_search.selected = selected;
    }
    pub(in crate::play) fn depot_search_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::depot_search::apply_result(&mut lock(&self.0).depot_search, response);
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    pub(crate) fn bevy_depot_search_initialize(&self) {
        self.depot_search_initialize(true);
    }
    pub(crate) fn bevy_depot_search_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.depot_search_result(response);
    }
}
