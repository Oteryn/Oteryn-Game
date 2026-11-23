use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
#[cfg(bevy_shell)]
use oteryn_client_runtime::container_tree::ContainerView;
use oteryn_client_runtime::inventory::MoveRequest;
use oteryn_session::ItemMoveDestination;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    /// Move the displayed source contents, in order, stopping at the first refusal.
    pub fn container_move_all_to(&self, destination_view: u8) -> bool {
        begin(&mut lock(&self.mailbox), &self.commands, destination_view)
    }
    /// Bind both displayed containers and the command to one current session mailbox snapshot.
    #[cfg(bevy_shell)]
    pub(crate) fn container_move_all_bound(
        &self,
        source: &ContainerView,
        destination: &ContainerView,
    ) -> bool {
        self.container_move_filtered_bound(source, destination, |_| true)
    }

    #[cfg(bevy_shell)]
    pub(crate) fn container_move_filtered_bound(
        &self,
        source: &ContainerView,
        destination: &ContainerView,
        include: impl Fn(&oteryn_session::ItemEntry) -> bool,
    ) -> bool {
        let included: std::collections::BTreeSet<_> = source
            .entries
            .iter()
            .filter(|entry| include(entry))
            .map(|entry| entry.handle)
            .collect();
        let mut mailbox = lock(&self.mailbox);
        if mailbox.world_input_blocked() || mailbox.server_modal.is_active() {
            return false;
        }
        let Some(views) = &mailbox.containers.views else {
            return false;
        };
        if !views.views.iter().any(|view| view == source)
            || !views.views.iter().any(|view| view == destination)
        {
            return false;
        }
        begin_for(
            &mut mailbox,
            &self.commands,
            source.view_id,
            destination.view_id,
            |entry| included.contains(&entry.handle),
        )
    }
}

pub(in crate::play) fn begin(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    destination_view: u8,
) -> bool {
    let Some(source) = mailbox.containers.active else {
        return false;
    };
    begin_for(mailbox, commands, source, destination_view, |_| true)
}
fn begin_for(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    source_view: u8,
    destination_view: u8,
    include: impl Fn(&oteryn_session::ItemEntry) -> bool,
) -> bool {
    let movement = &mailbox.item_movement;
    if mailbox.inventory_shutdown
        || mailbox.containers.bulk.pending
        || mailbox.containers.pending
        || movement.pending
        || movement.dragged.is_some()
        || movement.prompt.is_some()
    {
        return false;
    }
    let Some(source) = mailbox
        .containers
        .views
        .as_ref()
        .and_then(|views| views.views.iter().find(|view| view.view_id == source_view))
    else {
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
    {
        return false;
    }
    let requests: Vec<_> = source
        .entries
        .iter()
        .filter(|entry| include(entry))
        .map(|entry| MoveRequest {
            source: entry.handle,
            destination: ItemMoveDestination::Container(destination.container_handle, None),
            count: None,
        })
        .collect();
    if requests.is_empty()
        || requests
            .iter()
            .any(|request| request.source == destination.container_handle)
    {
        return false;
    }
    if commands
        .send(PlayCommand::MoveContainerContents(requests))
        .is_err()
    {
        mailbox.item_movement.feedback = Some(tr!("Session ended"));
        return false;
    }
    mailbox.item_movement.pending = true;
    mailbox.item_movement.feedback = Some("Moving container contents; stops at the first refusal");
    true
}
