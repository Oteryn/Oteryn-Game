use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn team_finder_advertise(
        &self,
        listing: Option<oteryn_client_ui::team_finder_leader::LeaderListing>,
    ) -> bool {
        let command = oteryn_client_ui::team_finder_leader::protocol::request(
            &mut lock(&self.mailbox).discovery.leader,
            listing,
        );
        command.is_some_and(|command| self.discovery_send(command))
    }
}
