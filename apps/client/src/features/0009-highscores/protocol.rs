use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn highscores_initialize(&self, selected: bool, loyalty: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.highscores.selected = selected;
        mailbox.highscores.loyalty_selected = loyalty;
    }
    pub(in crate::play) fn highscores_catalogue_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.highscores;
        panel.catalogue_selected = selected;
        panel.catalogue_pending = false;
        panel.catalogue_loaded = false;
        panel.catalogue.clear();
        if selected {
            panel.reply = None;
        } else {
            panel.world_type = None;
        }
    }
    pub(in crate::play) fn highscores_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        let mut mailbox = lock(&self.0);
        if mailbox.highscores.catalogue_selected {
            oteryn_client_ui::highscores_catalogue::apply_result(&mut mailbox.highscores, response);
        } else {
            oteryn_client_ui::highscores::apply_result(&mut mailbox.highscores, response);
        }
    }
}
