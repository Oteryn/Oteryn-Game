//! Transient destination selection over the retained, ordered container-transfer owner.
use super::*;
use bevy::{input_focus::tab_navigation::TabGroup, ui_widgets::ScrollArea};
use oteryn_client_ui::stash::{Batch, BatchKind, container_batch::Request};
use oteryn_client_ui_bevy as source;
use oteryn_session::container_tree::ContainerView;
#[derive(Resource, Default)]
struct Picker {
    next: u64,
    pending: Option<Selection>,
}
struct Selection {
    id: u64,
    generation: u64,
    source: ContainerView,
    destinations: Vec<ContainerView>,
    move_nested: bool,
    managed: bool,
}
#[derive(Component)]
struct Open {
    generation: u64,
    source: ContainerView,
}
#[derive(Component)]
struct Choice {
    id: u64,
    action: Action,
}
#[derive(Clone, Copy)]
enum Action {
    Destination(usize),
    Preview,
    Confirm(std::num::NonZeroU64),
    Close,
}
#[derive(Component)]
struct Root;
pub(super) fn install(app: &mut App) {
    app.init_resource::<Picker>().add_systems(
        Update,
        (escape, project).chain().after(super::view::project),
    );
}
// container.qml uses a configuration action in the sidebar header. No item-name/art inference.
pub(super) fn header(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    generation: u64,
    source: &ContainerView,
) {
    if source.browse.is_some() || source.entries.is_empty() {
        return;
    }
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 84.),
            Name::new("inventory.bulk.open"),
            Open {
                generation,
                source: source.clone(),
            },
        ))
        .observe(open)
        .id();
    let caption = commands
        .spawn(source::button_caption("Move contents…", skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
}
fn allowed(owner: &HostOwner) -> bool {
    owner.phase == crate::bevy_shell::owner::Phase::Playing
        && !owner.modal_active()
        && owner
            .play
            .as_ref()
            .is_some_and(|play| !play.view.gameplay_input_blocked(&play.link))
}
fn open(
    event: On<Activate>,
    buttons: Query<&Open>,
    owner: NonSend<HostOwner>,
    ui: Res<InventoryUi>,
    mut picker: ResMut<Picker>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity)
        || !allowed(&owner)
        || ui.menu.is_some()
        || ui.prompt.is_some()
        || ui.dragged.is_some()
    {
        return;
    }
    let Ok(button) = buttons.get(event.entity) else {
        return;
    };
    if button.generation != owner.generation() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.container_panel();
    let movement = play.link.item_movement();
    if panel.pending
        || panel.bulk.pending
        || movement.pending
        || movement.dragged.is_some()
        || movement.prompt.is_some()
    {
        return;
    }
    let Some(views) = panel.views else {
        return;
    };
    if !views.views.iter().any(|view| view == &button.source) {
        return;
    }
    let move_nested = ui
        .windows
        .get(&button.source.view_id)
        .is_none_or(|window| window.move_nested);
    let destinations = views
        .views
        .into_iter()
        .filter(|view| {
            view.browse.is_none()
                && view.container_handle != button.source.container_handle
                && !button.source.entries.iter().any(|item| {
                    item.handle == view.container_handle
                        && (move_nested
                            || play.view.scene().container_flag(item.appearance_id) == Some(false))
                })
        })
        .collect::<Vec<_>>();
    if destinations.is_empty() && !play.link.stash_panel().managed_selected {
        return;
    }
    let Some(id) = picker.next.checked_add(1) else {
        return;
    };
    play.link.cancel_managed_move();
    picker.next = id;
    picker.pending = Some(Selection {
        id,
        generation: owner.generation(),
        source: button.source.clone(),
        destinations,
        move_nested,
        managed: false,
    });
}
fn choose(
    event: On<Activate>,
    choices: Query<&Choice>,
    owner: NonSend<HostOwner>,
    mut picker: ResMut<Picker>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) || !allowed(&owner) {
        return;
    }
    let Ok(choice) = choices.get(event.entity) else {
        return;
    };
    if picker.pending.as_ref().is_none_or(|selection| {
        selection.id != choice.id || selection.generation != owner.generation()
    }) {
        return;
    }
    let (Some(selection), Some(play)) = (&mut picker.pending, &owner.play) else {
        return;
    };
    match choice.action {
        Action::Preview => {
            if selection.managed {
                return;
            }
            selection.managed = play.link.managed_move_action(
                &selection.source,
                Request::Preview {
                    source: selection.source.container_handle,
                    recursive: selection.move_nested,
                },
            );
        }
        Action::Confirm(preview) => {
            if selection.managed {
                play.link
                    .managed_move_action(&selection.source, Request::Confirm { preview });
            }
        }
        Action::Destination(index) => {
            if selection.managed {
                return;
            }
            if let Some(destination) = selection.destinations.get(index) {
                if selection.move_nested {
                    play.link
                        .container_move_all_bound(&selection.source, destination);
                } else {
                    play.link.container_move_filtered_bound(
                        &selection.source,
                        destination,
                        |item| play.view.scene().container_flag(item.appearance_id) == Some(false),
                    );
                }
            }
            picker.pending = None;
        }
        Action::Close => {
            play.link.cancel_managed_move();
            picker.pending = None;
        }
    }
}
fn escape(
    keys: Res<ButtonInput<KeyCode>>,
    owner: NonSend<HostOwner>,
    roots: Query<Entity, With<Root>>,
    guard: ModalGuard,
    mut picker: ResMut<Picker>,
) {
    if keys.just_pressed(KeyCode::Escape) && roots.iter().any(|entity| guard.allows(entity)) {
        if let Some(play) = &owner.play {
            play.link.cancel_managed_move();
        }
        picker.pending = None;
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut picker: ResMut<Picker>,
    roots: Query<Entity, With<Root>>,
    mut previous: Local<Option<(u64, Option<Batch>)>>,
) {
    if picker
        .pending
        .as_ref()
        .is_some_and(|selection| selection.generation != owner.generation() || !allowed(&owner))
    {
        picker.pending = None;
    }
    let stash = owner
        .play
        .as_ref()
        .filter(|_| picker.pending.is_some())
        .map(|play| play.link.stash_panel());
    let id = picker.pending.as_ref().map(|selection| {
        (
            selection.id,
            stash
                .as_ref()
                .filter(|_| selection.managed)
                .map(|panel| panel.managed_batch.clone()),
        )
    });
    if *previous == id {
        return;
    }
    previous.clone_from(&id);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some(selection) = &picker.pending else {
        return;
    };
    let overlay = commands
        .spawn((
            Root,
            super::super::GameModal,
            GlobalZIndex(90),
            Node {
                width: percent(100.),
                height: percent(100.),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.4)),
        ))
        .id();
    let panel = commands
        .spawn((
            ScrollArea,
            TabGroup {
                order: 4,
                modal: true,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(280.),
                max_width: percent(95.),
                max_height: percent(95.),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(10.), px(24.)),
                row_gap: px(4.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    let title = match (selection.managed, selection.move_nested) {
        (true, true) => "Move contents and nested items",
        (true, false) => "Move contents",
        (false, _) => "Move contents to",
    };
    source::dialog_caption(&mut commands, panel, title, skins.font.clone());
    if let Some(batch) = id.and_then(|(_, batch)| batch) {
        if let Some(description) = batch.description(BatchKind::ManagedMove) {
            let label = commands
                .spawn(source::text(
                    description,
                    source::font(skins.font.clone(), false),
                ))
                .id();
            commands.entity(panel).add_child(label);
        }
        if let Batch::Quote { id, .. } = batch {
            button(
                &mut commands,
                &skins,
                panel,
                selection.id,
                Action::Confirm(id),
                "Move stacks",
            );
        }
    } else {
        if stash.as_ref().is_some_and(|panel| panel.managed_selected) {
            button(
                &mut commands,
                &skins,
                panel,
                selection.id,
                Action::Preview,
                "Preview managed move",
            );
        }
        for (index, destination) in selection.destinations.iter().enumerate() {
            // Same truthful fallback as the existing open-container panels.
            let caption = format!(
                "Backpack ({}/{})",
                destination.entries.len(),
                destination.capacity
            );
            button(
                &mut commands,
                &skins,
                panel,
                selection.id,
                Action::Destination(index),
                &caption,
            );
        }
    }
    button(
        &mut commands,
        &skins,
        panel,
        selection.id,
        Action::Close,
        "Close",
    );
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    id: u64,
    action: Action,
    caption: &str,
) {
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 240.),
            Choice { id, action },
        ))
        .observe(choose)
        .id();
    let name = match action {
        Action::Destination(_) => "inventory.bulk.destination",
        Action::Preview => "inventory.bulk.managed",
        Action::Confirm(_) => "inventory.bulk.confirm",
        Action::Close => "inventory.bulk.close",
    };
    commands.entity(button).insert(Name::new(name));
    let caption = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
}
