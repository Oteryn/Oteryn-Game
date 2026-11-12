use crate::play::{PlayLink, mailbox::lock};
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn inventory_select(&self, handle: oteryn_session::ItemHandle) -> bool {
        let mut mailbox = lock(&self.mailbox);
        !mailbox.inventory_shutdown && mailbox.inventory.select(handle)
    }

    pub fn inventory_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::inventory::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            let mut mailbox = lock(&self.mailbox);
            if !mailbox.inventory_shutdown {
                mailbox.inventory.open = true;
            }
        }
        true
    }
}

impl PlayLink {
    pub fn item_drag_begin(&self, item: oteryn_client_ui::inventory::movement::DragItem) -> bool {
        let mut mailbox = lock(&self.mailbox);
        !mailbox.inventory_shutdown && mailbox.item_movement.begin(item)
    }
    pub fn item_drag_abort(&self) {
        lock(&self.mailbox).item_movement.abort();
    }
    pub fn item_drag_drop(&self, destination: Option<oteryn_session::ItemMoveDestination>) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.inventory_shutdown {
            return;
        }
        let counted = mailbox.item_count_selected;
        let request = mailbox.item_movement.drop_on(destination, counted);
        send_move(&mut mailbox, &self.commands, request);
    }
    pub fn item_move_confirm(&self, count: Option<u32>) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.inventory_shutdown {
            return;
        }
        let request = if let Some(count) = count {
            mailbox.item_movement.confirm(count)
        } else {
            mailbox.item_movement.cancel();
            None
        };
        send_move(&mut mailbox, &self.commands, request);
    }
    pub fn item_movement(&self) -> oteryn_client_ui::inventory::movement::Movement {
        lock(&self.mailbox).item_movement.clone()
    }
}
pub(in crate::play) fn send_move(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    request: Option<oteryn_client_runtime::inventory::MoveRequest>,
) {
    if let Some(request) = request
        && commands
            .send(crate::combat_input::PlayCommand::MoveItem(request))
            .is_err()
    {
        mailbox.item_movement.pending = false;
        mailbox.item_movement.feedback = Some(tr!("Session ended"));
    }
}

impl crate::play::PlayView {
    pub fn item_floor(&self) -> i16 {
        self.floor
    }
    pub fn item_drag_source(
        &self,
        tile: oteryn_renderer::TileCoord,
    ) -> Option<oteryn_client_ui::inventory::movement::DragItem> {
        // The shell already captured the drawn actor; do not also drag an item underneath it.
        if self
            .creature_drag
            .source
            .is_some_and(|(source, _, _)| source == tile)
        {
            return None;
        }
        self.ground_items
            .iter()
            .rev()
            .filter(|t| t.tile == tile)
            .filter_map(|t| t.item)
            .filter(|item| item.floor() == self.floor)
            .chain(
                self.entities
                    .iter()
                    .rev()
                    .filter(|e| {
                        e.position.floor == self.floor
                            && e.position.x == tile.x
                            && e.position.y == tile.y
                    })
                    .filter_map(super::state::entity_item),
            )
            .find_map(|item| {
                super::super::feature_0014_container_tree::input::ground_handle(item).map(
                    |handle| oteryn_client_ui::inventory::movement::DragItem {
                        equipped: false,
                        handle,
                        count: item.quantity(),
                    },
                )
            })
    }
}
