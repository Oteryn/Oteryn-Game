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
        let mut mailbox = lock(&self.0);
        #[cfg(feature = "bevy-shell")]
        if mailbox.shared_frontend {
            apply_bound(&mut mailbox.highscores, response);
            return;
        }
        oteryn_client_ui::highscores::apply_result(&mut mailbox.highscores, response);
    }
}

/// Preserve the in-flight query when an older query's success payload reaches this view.
#[cfg(feature = "bevy-shell")]
fn apply_bound(
    panel: &mut oteryn_client_ui::highscores::HighscoresPanel,
    response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
) {
    let mut prepared = panel.clone();
    oteryn_client_ui::highscores::apply_result(&mut prepared, response);
    use oteryn_client_runtime::server_mechanic::CommandStatus;
    if matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && prepared.feedback.is_some()
    {
        return;
    }
    *panel = prepared;
}
