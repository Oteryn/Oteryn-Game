use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_ui::stash::{Batch, container_batch::Request};
use oteryn_session::{CharacterInventory, SessionEvent, container_tree::ContainerView};

impl Outbox {
    pub(in crate::play) fn inventory_initialize(
        &self,
        inventory: Option<CharacterInventory>,
        counted: bool,
    ) {
        let mut mailbox = lock(&self.0);
        mailbox.item_count_selected = counted;
        mailbox.inventory.apply(inventory);
    }

    /// Manual Sort Mode starts off in every session; 510 makes it available.
    pub(in crate::play) fn container_manual_sort_initialize(&self, selected: bool) {
        lock(&self.0).container_manual_sort =
            oteryn_client_ui::container_manual_sort::admitted(selected);
    }

    pub(in crate::play) fn inventory_end(&self) {
        self.equipment_initialize(false);
        let mut mailbox = lock(&self.0);
        mailbox.world_context.targeting = None;
        mailbox.use_with_pending = false;
        mailbox.inventory = Default::default();
        mailbox.item_movement = Default::default();
        mailbox.item_count_selected = false;
        mailbox.container_manual_sort = Default::default();
        mailbox.inventory_shutdown = true;
        // Public template references belong to this inventory session too.
        mailbox.discovery.inspection = Default::default();
    }
}

impl PlayLink {
    #[must_use]
    pub fn inventory_panel(&self) -> oteryn_client_ui::inventory::InventoryPanel {
        lock(&self.mailbox).inventory.clone()
    }
    pub fn managed_move_action(&self, source: &ContainerView, request: Request) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.stash.managed_selected
            || mailbox.item_movement.pending
            || mailbox.containers.pending
            || mailbox.containers.bulk.pending
            || matches!(request, Request::Preview { source: handle, .. }
                if handle != source.container_handle || source.browse.is_some()
                    || mailbox.containers.views.as_ref().is_none_or(|views| !views.views.contains(source)))
        {
            return false;
        }
        let batch = &mut mailbox.stash.managed_batch;
        let Some(command) = oteryn_client_ui::stash::managed_move(batch, request) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::Feature(command))
            .is_err()
        {
            *batch = Batch::Unavailable;
            return false;
        }
        true
    }

    pub fn cancel_managed_move(&self) {
        let mut mailbox = lock(&self.mailbox);
        let batch = &mut mailbox.stash.managed_batch;
        // Keep the in-flight slot until its reply. A new preview must not inherit that reply.
        if *batch != Batch::Waiting {
            *batch = Batch::Idle;
        }
    }
}

macro_rules! session_trait {
    ($($methods:tt)*) => {
        super::feature_0011_world_interaction::protocol::session_trait! {
            $($methods)*
            fn equipment_selected(&self) -> bool { false }
            fn item_count_selected(&self) -> bool { false }
            fn move_item(
                &mut self,
                _request: oteryn_client_runtime::inventory::MoveRequest,
            ) -> impl std::future::Future<
                Output = Result<
                    Option<oteryn_client_runtime::inventory::MoveResponse>,
                    oteryn_session::SessionError,
                >,
            > + Send {
                async { Ok(None) }
            }
        }
    };
}
pub(in crate::play) use session_trait;
macro_rules! dispatch {
    ($command:expr, $session:ident, $outbox:ident, $held:ident, $publish:ident; $($arms:tt)*) => {
        super::feature_0011_world_interaction::protocol::dispatch! {
            $command, $session, $outbox, $held, $publish;
            Ok(crate::combat_input::PlayCommand::Feature(command))
                if matches!(command.capability, 591 | 601 | 602) =>
            {
                match $outbox.stash_extension(&mut $session, &command).await {
                    Ok(()) => {
                        $publish(&mut $session, &$outbox, &mut $held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(crate::combat_input::PlayCommand::MoveContainerContents(requests)) => {
                $outbox.target_walk_replaced();
                let result = async {
                    let mut last = None;
                    for request in requests {
                        last = $session.move_item(request).await?;
                        let moved = last.is_some_and(|reply| reply.outcome == oteryn_session::ItemMoveOutcome::Moved);
                        $publish(&mut $session, &$outbox, &mut $held);
                        if !moved { break; }
                    }
                    Ok::<_, oteryn_session::SessionError>(last)
                }.await;
                match result {
                    Ok(response) => { $outbox.item_move_result(response); continue; }
                    Err(error) => error,
                }
            }
            Ok(crate::combat_input::PlayCommand::MoveItem(request)) => {
                $outbox.target_walk_replaced();
                match $session.move_item(request).await {
                    Ok(response) => {
                        $outbox.item_move_result(response);
                        $publish(&mut $session, &$outbox, &mut $held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            $($arms)*
        }
    };
}
pub(in crate::play) use dispatch;
impl Outbox {
    pub(in crate::play) fn item_move_result(
        &self,
        response: Option<oteryn_client_runtime::inventory::MoveResponse>,
    ) {
        let mut mailbox = lock(&self.0);
        mailbox.item_movement.pending = false;
        mailbox.item_movement.feedback = Some(oteryn_client_ui::localization::tr(
            oteryn_client_runtime::inventory::feedback(response),
        ));
    }
}

impl crate::play::mailbox::Mailbox {
    pub(in crate::play) fn inventory_events(&mut self, events: &[SessionEvent]) {
        for event in events {
            if let SessionEvent::Inventory(delta) = event {
                self.inventory.apply(Some(delta.value.clone()));
            }
            if let SessionEvent::InventoryCapacity(delta) = event {
                oteryn_client_ui::inventory_capacity::apply(&mut self.inventory, delta.value);
            }
        }
    }
}
