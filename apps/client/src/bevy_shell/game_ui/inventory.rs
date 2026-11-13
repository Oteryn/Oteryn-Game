//! Authoritative equipment and container presentation; the retained PlayLink owns all commands.
use super::super::{owner::HostOwner, ui::Skins};
use crate::bevy_shell::game_ui::modal::ModalGuard;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::inventory::movement::DragItem;
use oteryn_session::{ItemEntry, ItemMoveDestination, item_movement::EquipmentSlot};

mod actions;
mod amount;
mod dialogs;
#[cfg(test)]
mod tests;
mod view;
mod world_drag;
pub(in crate::bevy_shell) use world_drag::ItemPointerCapture;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Location {
    Inventory,
    Equipment(EquipmentSlot),
    Container {
        view: u8,
        handle: oteryn_session::ItemHandle,
    },
}
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct Slot {
    generation: u64,
    location: Location,
    item: Option<ItemEntry>,
    destination: Option<ItemMoveDestination>,
}
impl Slot {
    fn current(&self, owner: &HostOwner) -> Option<Option<ItemEntry>> {
        if owner.generation() != self.generation || owner.modal_active() {
            return None;
        }
        let play = owner.play.as_ref()?;
        if play.link.server_modal_active() {
            return None;
        }
        match self.location {
            Location::Inventory => {
                let panel = play.link.inventory_panel();
                panel.inventory.as_ref()?;
                match self.item {
                    Some(item) => panel
                        .rows()
                        .into_iter()
                        .find(|entry| entry.handle == item.handle)
                        .map(Some),
                    None => Some(None),
                }
            }
            Location::Equipment(slot) => {
                let slots = play.link.equipment_slots();
                slots
                    .iter()
                    .find(|entry| {
                        entry.available && entry.destination == ItemMoveDestination::Equipment(slot)
                    })
                    .map(|entry| entry.item)
            }
            Location::Container { view, handle } => {
                let panel = play.link.container_panel();
                let container = panel
                    .views
                    .as_ref()?
                    .views
                    .iter()
                    .find(|entry| entry.view_id == view && entry.container_handle == handle)?;
                match self.item {
                    Some(item) => container
                        .entries
                        .iter()
                        .find(|entry| entry.handle == item.handle)
                        .copied()
                        .map(Some),
                    None => Some(None),
                }
            }
        }
    }
    fn current_item(&self, owner: &HostOwner) -> Option<ItemEntry> {
        let current = self.current(owner)??;
        let old = self.item?;
        (current.handle == old.handle
            && current.item_definition_ref == old.item_definition_ref
            && current.sub_type == old.sub_type)
            .then_some(current)
    }
    fn drag(&self, owner: &HostOwner) -> Option<DragItem> {
        let item = self.current_item(owner)?;
        // The backpack equipment cell is a container destination, not a movable backpack.
        if self.location == Location::Inventory
            && self.destination == Some(ItemMoveDestination::MainBackpack)
        {
            return None;
        }
        Some(DragItem {
            equipped: matches!(self.location, Location::Equipment(_)),
            handle: item.handle,
            count: item.count.get(),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Prompt {
    generation: u64,
    item: DragItem,
    destination: ItemMoveDestination,
}
#[derive(Resource, Default)]
struct InventoryUi {
    selection: Option<Slot>,
    menu: Option<(Slot, Vec2)>,
    split_count: u32,
    split_text: Option<String>,
    split_feedback: String,
    split_entry: Option<ItemEntry>,
    pending_amount_confirm: Option<Prompt>,
    dragged: Option<Slot>,
    prompt: Option<Prompt>,
    ground_source: Option<world_drag::GroundSource>,
}
#[derive(Component, Clone, Copy)]
enum Control {
    Open(Slot),
    Context(Slot, oteryn_client_ui::world_interaction::menu::Action),
    Up {
        generation: u64,
        view: u8,
        handle: oteryn_session::ItemHandle,
    },
    Close {
        generation: u64,
        view: u8,
        handle: oteryn_session::ItemHandle,
    },
    Count(Prompt, i32),
    All(Prompt),
    Confirm(Prompt),
    Cancel {
        generation: u64,
        prompt: Option<Prompt>,
    },
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<InventoryUi>().add_systems(
        Update,
        (
            actions::escape.after(crate::bevy_shell::input::route),
            view::project,
        )
            .chain(),
    );
    amount::install(app);
    world_drag::install(app);
}
/// Minimal real state for raw-input caller fixtures; production always installs the full module.
#[cfg(test)]
pub(in crate::bevy_shell) fn initialize_item_pointer_test_state(app: &mut App) {
    app.init_resource::<InventoryUi>()
        .init_resource::<world_drag::Gesture>()
        .init_resource::<bevy::diagnostic::FrameCount>();
}

fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&Control>,
    fields: Query<(&amount::Field, &bevy::text::EditableText)>,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<InventoryUi>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        if let Control::Confirm(prompt) = *control
            && amount::prepare_confirm(prompt, &fields, &owner, &mut ui)
        {
            return;
        }
        actions::choose(*control, &owner, &mut ui);
    }
}

/// Action-bar assignments accept only an item still carried by this admitted session.
pub(super) fn current_drag_item(
    owner: &HostOwner,
    source: super::ItemDragSource,
) -> Option<ItemEntry> {
    if owner.generation() != source.generation || owner.modal_active() {
        return None;
    }
    let play = owner.play.as_ref()?;
    if play.link.server_modal_active() {
        return None;
    }
    let inventory = play.link.inventory_panel();
    let containers = play.link.container_panel();
    inventory
        .inventory
        .iter()
        .flat_map(|inventory| {
            inventory
                .main_backpack
                .iter()
                .chain(&inventory.entries)
                .chain(inventory.equipment.iter().map(|equipped| &equipped.item))
        })
        .chain(
            containers
                .views
                .iter()
                .flat_map(|views| &views.views)
                .flat_map(|view| &view.entries),
        )
        .find(|item| {
            item.handle == source.item.handle
                && item.item_definition_ref == source.item.item_definition_ref
                && item.sub_type == source.item.sub_type
        })
        .copied()
}
