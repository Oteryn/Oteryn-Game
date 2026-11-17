use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn highscores_initialize(&self, selected: bool, loyalty: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.highscores.selected = selected;
        mailbox.highscores.loyalty_selected = loyalty;
    }
    pub(in crate::play) fn highscores_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::highscores::apply_result(&mut lock(&self.0).highscores, response);
    }
}
