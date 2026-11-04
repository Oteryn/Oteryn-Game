use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn highscores_initialize(&self, selected: bool) {
        lock(&self.0).highscores.selected = selected;
    }
    pub(in crate::play) fn highscores_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::highscores::apply_result(&mut lock(&self.0).highscores, response);
    }
}
