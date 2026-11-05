use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn purchase_history(&self, next: bool) -> bool {
        let command = oteryn_client_ui::purchase_history::protocol::request(
            &mut lock(&self.mailbox).discovery.history,
            next,
        );
        command.is_some_and(|command| self.discovery_send(command))
    }
}
