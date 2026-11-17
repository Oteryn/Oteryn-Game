#[path = "input/bulk.rs"]
pub(super) mod bulk;
use crate::combat_input::PlayCommand;
use crate::play::{GroundItem, PlayLink, PlayView, mailbox::lock};
use oteryn_client_runtime::container_tree::{ContainerViewIntent, ItemHandle};
use oteryn_input_actions::NormalizedInputEvent;

pub(crate) fn ground_handle(item: GroundItem) -> Option<ItemHandle> {
    match item {
        GroundItem::Entity { handle, .. } => handle,
        GroundItem::Map { item, .. } => match item.origin {
            oteryn_session::world_map::MapOrigin::Handle(handle) => Some(handle),
            _ => None,
        },
    }
}
impl PlayView {
    pub fn container_input(&self, link: &PlayLink, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::container_tree::input::open_key(events) {
            return false;
        }
        if let Some(handle) = self
            .scene()
            .target()
            .and_then(|t| t.item)
            .and_then(ground_handle)
        {
            link.container_open(handle);
        }
        true
    }
}
impl PlayLink {
    pub fn inventory_open_container(&self) -> bool {
        let handle = {
            let mailbox = lock(&self.mailbox);
            if mailbox.inventory_shutdown {
                return false;
            }
            mailbox.inventory.selected_item().map(|item| item.handle)
        };
        handle.is_some_and(|handle| self.container_open(handle))
    }
    pub fn container_open(&self, handle: ItemHandle) -> bool {
        self.container_request(|panel| panel.open(handle, None))
    }
    pub fn container_open_selected(&self) -> bool {
        self.container_request(|panel| panel.open_selected())
    }
    pub fn container_up(&self) -> bool {
        self.container_request(|panel| panel.up())
    }
    pub fn container_close(&self) -> bool {
        self.container_request(|panel| panel.close())
    }
    /// Keep the clicked container identity and the emitted intent under one mailbox lock.
    pub fn container_up_bound(&self, view: u8, handle: ItemHandle) -> bool {
        self.container_request(|panel| {
            panel
                .views
                .as_ref()?
                .views
                .iter()
                .find(|entry| entry.view_id == view && entry.container_handle == handle)?
                .parent_handle?;
            if !panel.activate(view) {
                return None;
            }
            panel.up()
        })
    }
    pub fn container_close_bound(&self, view: u8, handle: ItemHandle) -> bool {
        self.container_request(|panel| {
            panel
                .views
                .as_ref()?
                .views
                .iter()
                .find(|entry| entry.view_id == view && entry.container_handle == handle)?;
            if !panel.activate(view) {
                return None;
            }
            panel.close()
        })
    }
    #[cfg(all(test, feature = "bevy-shell"))]
    pub(crate) fn bevy_items_fixture(
        outbox: &crate::play::Outbox,
        inventory: Option<oteryn_session::CharacterInventory>,
        views: Option<oteryn_client_runtime::container_tree::ContainerViews>,
        counted: bool,
        equipment: bool,
    ) {
        outbox.inventory_initialize(inventory, counted);
        outbox.equipment_initialize(equipment);
        outbox.container_initialize(views);
        outbox.world_context_initialize(true, false);
        outbox.targeting_initialize(true);
    }
    pub fn container_select(&self, view: u8, handle: ItemHandle) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.containers.active != Some(view) && !mailbox.containers.activate(view) {
            return false;
        }
        mailbox.containers.select(handle)
    }
    pub fn container_activate(&self, view: u8) -> bool {
        lock(&self.mailbox).containers.activate(view)
    }
    fn container_request(
        &self,
        request: impl FnOnce(
            &mut oteryn_client_ui::container_tree::ContainerPanel,
        ) -> Option<ContainerViewIntent>,
    ) -> bool {
        let sent = {
            let mut mailbox = lock(&self.mailbox);
            let Some(intent) = request(&mut mailbox.containers) else {
                return false;
            };
            if self
                .commands
                .send(PlayCommand::ContainerView(intent))
                .is_err()
            {
                mailbox.containers.end();
                return false;
            }
            true
        };
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).containers.open = true;
        }
        sent
    }
}

#[cfg(any(windows, test))]
pub(in crate::play) fn inventory_native_open(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    handle: ItemHandle,
) -> bool {
    let Some(intent) = mailbox.containers.open(handle, None) else {
        return false;
    };
    if commands.send(PlayCommand::ContainerView(intent)).is_err() {
        mailbox.containers.end();
        return false;
    }
    true
}
