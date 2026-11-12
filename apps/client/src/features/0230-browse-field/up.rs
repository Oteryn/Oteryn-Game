//! Crystal 201189f game.cpp:4455-4499: Up keeps the existing window id;
//! the authoritative owner decides whether its parent is another bag or a browse field.
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{SharedMailbox, lock},
    },
};
use oteryn_session::container_tree::{ContainerViewIntent, encode_container_view_intent};
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn browse_field_up(&self, view_id: u8) {
        enqueue_up(&self.mailbox, &self.commands, view_id);
    }
}
pub(in crate::play) fn enqueue_up(
    mailbox: &SharedMailbox,
    commands: &UnboundedSender<PlayCommand>,
    view_id: u8,
) {
    let Ok(payload) = encode_container_view_intent(&ContainerViewIntent::Up { view_id }) else {
        return;
    };
    let mut m = lock(mailbox);
    let command = oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
        capability: 14,
        command_type: 21,
        payload,
    };
    if let Some(command) = oteryn_client_ui::browse_field::request(&mut m.browse_field, command)
        && commands.send(PlayCommand::ServerMechanic(command)).is_err()
    {
        m.browse_field.pending = false;
        m.browse_field.feedback = Some(tr!("The session has ended").into());
    }
}
