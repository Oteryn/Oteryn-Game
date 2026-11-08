use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_session::{CharacterInventory, SessionEvent};

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

    pub(in crate::play) fn inventory_events(&self, events: &[SessionEvent]) {
        let mut mailbox = lock(&self.0);
        for event in events {
            if let SessionEvent::Inventory(delta) = event {
                mailbox.inventory.apply(Some(delta.value.clone()));
            }
        }
    }

    pub(in crate::play) fn inventory_end(&self) {
        self.equipment_initialize(false);
        let mut mailbox = lock(&self.0);
        mailbox.world_context.targeting = None;
        mailbox.inventory = Default::default();
        mailbox.item_movement = Default::default();
        mailbox.item_count_selected = false;
        mailbox.inventory_shutdown = true;
    }
}

impl PlayLink {
    #[must_use]
    pub fn inventory_panel(&self) -> oteryn_client_ui::inventory::InventoryPanel {
        lock(&self.mailbox).inventory.clone()
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
            Ok(crate::combat_input::PlayCommand::MoveContainerContents(requests)) => {
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
        mailbox.item_movement.feedback = Some(oteryn_client_runtime::inventory::feedback(response));
    }
}
