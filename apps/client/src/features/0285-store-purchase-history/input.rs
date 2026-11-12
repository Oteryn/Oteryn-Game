use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn purchase_history_filter(&self, offer: &str, from: &str, until: &str) -> bool {
        let mut mailbox = lock(&self.mailbox);
        mailbox.store.page = oteryn_client_ui::store::StorePage::History;
        oteryn_client_ui::purchase_history::set_filter(
            &mut mailbox.discovery.history,
            offer,
            from,
            until,
        )
    }
    pub fn purchase_history(&self, next: bool) -> bool {
        let command = oteryn_client_ui::purchase_history::protocol::request(
            &mut lock(&self.mailbox).discovery.history,
            next,
        );
        command.is_some_and(|command| self.discovery_send(command))
    }
}
