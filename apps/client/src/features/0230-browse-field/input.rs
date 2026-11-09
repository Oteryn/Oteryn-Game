use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn browse_field_request(&self, payload: Vec<u8>) {
        let mut mailbox = lock(&self.mailbox);
        let command = oteryn_client_ui::browse_field::command(payload);
        if let Some(command) =
            oteryn_client_ui::browse_field::request(&mut mailbox.browse_field, command)
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            mailbox.browse_field.pending = false;
            mailbox.browse_field.feedback = Some(tr!("The session has ended").into());
        }
    }
    pub fn browse_field_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        if !oteryn_client_ui::browse_field::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).browse_field.open = true;
        }
        true
    }
}

impl PlayLink {
    pub fn browse_field_move(
        &self,
        source: oteryn_session::ItemHandle,
        destination: oteryn_session::ItemMoveDestination,
    ) {
        enqueue_move(&self.mailbox, &self.commands, source, destination);
    }
}
pub(in crate::play) fn enqueue_move(
    mailbox: &crate::play::mailbox::SharedMailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    source: oteryn_session::ItemHandle,
    destination: oteryn_session::ItemMoveDestination,
) {
    let mut m = lock(mailbox);
    if !m.browse_field.selected || m.browse_field.pending {
        return;
    }
    m.browse_field.pending = true;
    m.browse_field.feedback = None;
    if commands
        .send(PlayCommand::BrowseFieldMove(
            oteryn_session::ItemMoveIntent {
                source,
                destination,
            },
        ))
        .is_err()
    {
        m.browse_field.pending = false;
        m.browse_field.feedback = Some(tr!("The session has ended").into());
    }
}

#[path = "up.rs"]
mod up;
