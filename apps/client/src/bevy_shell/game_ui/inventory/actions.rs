use super::*;
use crate::bevy_shell::game_ui::modal::ModalGuard;
use bevy::picking::events::{PointerClick, PointerDragDrop, PointerDragStart};
pub(super) fn escape(
    guard: ModalGuard,
    overlays: Query<Entity, (With<super::super::GameModal>, With<super::view::Projection>)>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<InventoryUi>,
) {
    let server_modal = owner
        .play
        .as_ref()
        .is_some_and(|play| play.link.server_modal_active());
    let allowed = guard.allows_focus(None) || overlays.iter().any(|entity| guard.allows(entity));
    if keys.just_pressed(KeyCode::Escape) && !server_modal && allowed {
        ui.menu = None;
        if let Some(play) = &owner.play {
            if play.link.item_movement().prompt.is_some() {
                play.link.item_move_confirm(None);
            }
            play.link.item_drag_abort();
        }
        ui.dragged = None;
    }
    let lost_focus = windows.single().is_ok_and(|window| !window.focused);
    if ui.dragged.is_some_and(|slot| {
        !buttons.pressed(MouseButton::Left)
            || lost_focus
            || server_modal
            || slot.current_item(&owner).is_none()
    }) {
        if let Some(play) = &owner.play {
            play.link.item_drag_abort();
        }
        ui.dragged = None;
    }
}
pub(super) fn select(
    event: On<Activate>,
    guard: ModalGuard,
    slots: Query<&Slot>,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<InventoryUi>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if ui.prompt.is_none()
        && ui.menu.is_none()
        && let Ok(slot) = slots.get(event.entity)
    {
        select_slot(*slot, &owner, &mut ui);
    }
}
pub(super) fn click(
    event: On<PointerClick>,
    guard: ModalGuard,
    slots: Query<&Slot>,
    owner: NonSend<HostOwner>,
    scale: Res<UiScale>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut ui: ResMut<InventoryUi>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    use bevy::picking::pointer::PointerButton;
    if ui.prompt.is_some() {
        return;
    }
    let Ok(slot) = slots.get(event.entity) else {
        return;
    };
    if slot.current_item(&owner).is_none() {
        return;
    }
    if event.button == PointerButton::Secondary {
        select_slot(*slot, &owner, &mut ui);
        let mut position = event.pointer.position / scale.0;
        if let Ok(window) = windows.single() {
            position.x = position
                .x
                .clamp(0., (window.width() / scale.0 - 145.).max(0.));
            position.y = position
                .y
                .clamp(0., (window.height() / scale.0 - 150.).max(0.));
        }
        ui.menu = Some((*slot, position));
    } else if event.button == PointerButton::Primary && event.count >= 2 {
        let backpack = owner
            .play
            .as_ref()
            .and_then(|play| play.link.inventory_panel().inventory)
            .and_then(|inventory| inventory.main_backpack);
        let control = if slot.item.is_some() && slot.item == backpack {
            Control::Open(*slot)
        } else {
            Control::Context(
                *slot,
                oteryn_client_ui::world_interaction::menu::Action::Use,
            )
        };
        choose(control, &owner, &mut ui);
    }
}
pub(super) fn drag_start(
    event: On<PointerDragStart>,
    guard: ModalGuard,
    slots: Query<&Slot>,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<InventoryUi>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if event.button == bevy::picking::pointer::PointerButton::Primary
        && ui.prompt.is_none()
        && ui.menu.is_none()
        && let Ok(slot) = slots.get(event.entity)
    {
        begin_slot(*slot, &owner, &mut ui);
    }
}
pub(super) fn drag_drop(
    event: On<PointerDragDrop>,
    guard: ModalGuard,
    slots: Query<&Slot>,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<InventoryUi>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if event.button != bevy::picking::pointer::PointerButton::Primary {
        return;
    }
    if let (Ok(source), Ok(target)) = (slots.get(event.dropped), slots.get(event.entity)) {
        drop_slot(*source, *target, &owner, &mut ui);
    }
}

pub(super) fn choose(control: Control, owner: &HostOwner, ui: &mut InventoryUi) {
    if owner
        .play
        .as_ref()
        .is_some_and(|play| play.link.server_modal_active())
    {
        return;
    }
    match control {
        Control::Open(slot) => {
            if let Some(item) = slot.current_item(owner)
                && let Some(play) = &owner.play
            {
                play.link.container_open(item.handle);
                ui.menu = None;
            }
        }
        Control::Context(slot, action) => {
            if let Some(item) = slot.current_item(owner)
                && let Some(play) = &owner.play
            {
                use oteryn_client_ui::world_interaction::menu::{Menu, Target};
                let menu = Menu::new(Target::Item(item.handle), true, false).with_targeting(true);
                // The retained view revalidates every action against negotiated capabilities.
                play.view.world_context_choose(&play.link, &menu, action);
                ui.menu = None;
            }
        }
        Control::Up {
            generation,
            view,
            handle,
        }
        | Control::Close {
            generation,
            view,
            handle,
        } if generation == owner.generation() && !owner.modal_active() => {
            if let Some(play) = &owner.play {
                if matches!(control, Control::Up { .. }) {
                    play.link.container_up_bound(view, handle);
                } else {
                    play.link.container_close_bound(view, handle);
                }
            }
        }
        Control::Count(prompt, delta) if prompt_current(prompt, owner, ui) => {
            super::amount::change_count(owner, ui, prompt, Some(delta));
        }
        Control::All(prompt) if prompt_current(prompt, owner, ui) => {
            super::amount::change_count(owner, ui, prompt, None);
        }
        Control::Confirm(prompt) => super::amount::confirm(owner, ui, prompt),
        Control::Cancel { generation, prompt } if generation == owner.generation() => {
            if let Some(prompt) = prompt {
                if prompt_current(prompt, owner, ui)
                    && let Some(play) = &owner.play
                {
                    play.link.item_move_confirm(None);
                }
            } else {
                ui.menu = None;
            }
        }
        _ => {}
    }
}
pub(super) fn prompt_current(prompt: Prompt, owner: &HostOwner, ui: &InventoryUi) -> bool {
    prompt.generation == owner.generation()
        && ui.prompt == Some(prompt)
        && !owner.modal_active()
        && owner.play.as_ref().is_some_and(|play| {
            play.link.item_movement().prompt == Some((prompt.item, prompt.destination))
        })
}
fn select_slot(slot: Slot, owner: &HostOwner, ui: &mut InventoryUi) {
    let Some(item) = slot.current_item(owner) else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    match slot.location {
        Location::Inventory => {
            play.link.inventory_select(item.handle);
        }
        Location::Container { view, .. } => {
            play.link.container_select(view, item.handle);
        }
        Location::Equipment(_) => {}
    }
    ui.selection = Some(slot);
}
fn begin_slot(slot: Slot, owner: &HostOwner, ui: &mut InventoryUi) {
    if let Some(item) = slot.drag(owner)
        && let Some(play) = &owner.play
        && play.link.item_drag_begin(item)
    {
        ui.dragged = Some(slot);
        ui.menu = None;
    }
}
fn drop_slot(source: Slot, target: Slot, owner: &HostOwner, ui: &mut InventoryUi) {
    if ui.dragged != Some(source) {
        return;
    }
    ui.dragged = None;
    let Some(play) = &owner.play else {
        return;
    };
    let destination = target.current(owner).and(target.destination);
    if let Some(item) = source.drag(owner)
        && destination.is_some()
        && play.link.item_drag_begin(item)
    {
        play.link.item_drag_drop(destination);
    } else {
        play.link.item_drag_abort();
    }
}
