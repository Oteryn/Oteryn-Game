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
        let mut position = event.pointer.position;
        // The same capability flags decide which rows the menu shows (view::project).
        let extra = owner.play.as_ref().map_or((false, false), |play| {
            (
                play.link.mail_panel().selected,
                play.link.timed_item_panel().selected,
            )
        });
        if let Ok(window) = windows.single() {
            position.x = position
                .x
                .clamp(0., (window.width() - 145. * scale.0).max(0.));
            position.y = position.y.clamp(
                0.,
                (window.height() - super::dialogs::menu_height(extra) * scale.0).max(0.),
            );
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
    capture: super::world_drag::DropCapture,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if event.button == bevy::picking::pointer::PointerButton::Primary
        && ui.prompt.is_none()
        && ui.menu.is_none()
        && let Ok(slot) = slots.get(event.entity)
        && capture.admits_source(*slot, event.pointer.id, event.original_event_target())
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
    mut capture: super::world_drag::DropCapture,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if event.button != bevy::picking::pointer::PointerButton::Primary
        || !capture.matches_pointer(event.pointer.id)
    {
        return;
    }
    if let Ok(target) = slots.get(event.entity) {
        if !capture.slot(&event, &owner, *target) {
            return;
        }
        if let Ok(source) = slots.get(event.dropped) {
            drop_slot(*source, *target, &owner, &mut ui);
            capture.completed();
        }
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
        Control::Write(slot) | Control::Timed(slot) => {
            if let Some(item) = slot.current_item(owner)
                && let Some(play) = &owner.play
            {
                if matches!(control, Control::Write(_)) {
                    play.link.mail_write_item(item.handle);
                } else {
                    play.link.timed_item_inspect(item.handle);
                }
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
        Control::GroundUp {
            generation,
            view,
            handle,
        } if generation == owner.generation() && !owner.modal_active() => {
            if let Some(play) = &owner.play {
                play.link.browse_field_up_bound(view, handle);
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
        ui.ground_source = None;
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
    let destination = target
        .current(owner)
        .and(target.destination)
        .filter(|destination| {
            !matches!(destination, ItemMoveDestination::Ground(_))
                || super::world_drag::world_input_allowed(owner)
        });
    if let Some(item) = source.drag(owner)
        && destination.is_some()
        && play.link.item_drag_begin(item)
    {
        play.link.item_drag_drop(destination);
    } else {
        play.link.item_drag_abort();
    }
}

#[cfg(test)]
#[test]
fn inventory_right_click_places_popup_at_scaled_cursor() -> Result<(), Box<dyn std::error::Error>> {
    use crate::bevy_shell::gameplay_dialogs::popup;
    use bevy::picking::{backend::HitData, pointer::PointerButton};
    let (link, _, outbox) = crate::play::play_channel();
    let item = super::tests::entry(7, 3)?;
    outbox.push(vec![oteryn_session::SessionEvent::Inventory(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::CharacterInventory {
                main_backpack: None,
                entries: vec![item],
                equipment: vec![],
            },
        },
    )]);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .insert_resource(UiScale(2.))
        .init_resource::<InventoryUi>()
        .insert_non_send(super::tests::host(link)?)
        .add_systems(
            Startup,
            (crate::bevy_shell::ui::setup, super::super::setup).chain(),
        )
        .add_systems(
            Update,
            (super::super::project, super::view::project, popup::layout).chain(),
        );
    app.world_mut().spawn((
        Window {
            resolution: (1024, 768).into(),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.update();
    let entity = app
        .world_mut()
        .query::<(Entity, &Slot)>()
        .iter(app.world())
        .find_map(|(entity, slot)| (slot.item == Some(item)).then_some(entity))
        .ok_or("occupied slot")?;
    let mut pointer = super::tests::pointer();
    pointer.position = Vec2::new(400., 200.);
    app.world_mut().trigger(PointerClick {
        entity,
        pointer,
        button: PointerButton::Secondary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        duration: std::time::Duration::ZERO,
        count: 1,
    });
    app.update();
    let node = app
        .world_mut()
        .query_filtered::<&Node, With<popup::PopupMenu>>()
        .single(app.world())?;
    assert_eq!((node.left, node.top), (px(200.), px(100.)));
    Ok(())
}
