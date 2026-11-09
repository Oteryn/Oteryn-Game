use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_runtime::inventory::MoveRequest;
use oteryn_session::ItemMoveDestination;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    /// Move the displayed source contents, in order, stopping at the first refusal.
    pub fn container_move_all_to(&self, destination_view: u8) -> bool {
        begin(&mut lock(&self.mailbox), &self.commands, destination_view)
    }
}

pub(in crate::play) fn begin(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    destination_view: u8,
) -> bool {
    let movement = &mailbox.item_movement;
    if mailbox.inventory_shutdown
        || mailbox.containers.pending
        || movement.pending
        || movement.dragged.is_some()
        || movement.prompt.is_some()
    {
        return false;
    }
    let Some(source) = mailbox.containers.current() else {
        return false;
    };
    let Some(destination) = mailbox.containers.views.as_ref().and_then(|views| {
        views
            .views
            .iter()
            .find(|view| view.view_id == destination_view)
    }) else {
        return false;
    };
    if source.browse.is_some()
        || destination.browse.is_some()
        || source.container_handle == destination.container_handle
        || source.entries.is_empty()
        || source.entries.len() > 255
        || source
            .entries
            .iter()
            .any(|entry| entry.handle == destination.container_handle)
    {
        return false;
    }
    let requests = source
        .entries
        .iter()
        .map(|entry| MoveRequest {
            source: entry.handle,
            destination: ItemMoveDestination::Container(destination.container_handle),
            count: None,
        })
        .collect();
    if commands
        .send(PlayCommand::MoveContainerContents(requests))
        .is_err()
    {
        mailbox.item_movement.feedback = Some("Session ended");
        return false;
    }
    mailbox.item_movement.pending = true;
    mailbox.item_movement.feedback = Some("Moving container contents; stops at the first refusal");
    true
}
