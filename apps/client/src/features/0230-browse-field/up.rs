//! Crystal 201189f game.cpp:4455-4499: Up keeps the existing window id;
//! the authoritative owner decides whether its parent is another bag or a browse field.
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, SharedMailbox, lock},
    },
};
use oteryn_session::{
    ItemHandle,
    container_tree::{ContainerViewIntent, encode_container_view_intent},
};
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn browse_field_up(&self, view_id: u8) {
        enqueue_up(&self.mailbox, &self.commands, view_id);
    }
    /// Up from a container lying on the ground (Tibia Global opens that tile's browse field).
    /// The clicked view is re-proved under the same mailbox lock that emits the intent.
    pub fn browse_field_up_bound(&self, view_id: u8, handle: ItemHandle) -> bool {
        let mut m = lock(&self.mailbox);
        let bound = m.containers.views.as_ref().is_some_and(|views| {
            views.views.iter().any(|view| {
                view.view_id == view_id
                    && view.container_handle == handle
                    && view.parent_handle.is_none()
                    && view.browse.is_none()
            })
        });
        bound && send_up(&mut m, &self.commands, view_id)
    }
}
pub(in crate::play) fn enqueue_up(
    mailbox: &SharedMailbox,
    commands: &UnboundedSender<PlayCommand>,
    view_id: u8,
) {
    send_up(&mut lock(mailbox), commands, view_id);
}
fn send_up(m: &mut Mailbox, commands: &UnboundedSender<PlayCommand>, view_id: u8) -> bool {
    let Ok(payload) = encode_container_view_intent(&ContainerViewIntent::Up { view_id }) else {
        return false;
    };
    let command = oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
        capability: 14,
        command_type: 21,
        payload,
    };
    let Some(command) = oteryn_client_ui::browse_field::request(&mut m.browse_field, command)
    else {
        return false;
    };
    if commands.send(PlayCommand::ServerMechanic(command)).is_err() {
        m.browse_field.pending = false;
        m.browse_field.feedback = Some(tr!("The session has ended").into());
        return false;
    }
    true
}
