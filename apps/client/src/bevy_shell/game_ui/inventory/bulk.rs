//! Transient destination selection over the retained, ordered container-transfer owner.
use super::*;
use bevy::{input_focus::tab_navigation::TabGroup, ui_widgets::ScrollArea};
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
}
#[derive(Component)]
struct Open {
    generation: u64,
    source: ContainerView,
}
#[derive(Component)]
struct Choice {
    id: u64,
    destination: Option<usize>,
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
    let destinations = views
        .views
        .into_iter()
        .filter(|view| {
            view.browse.is_none()
                && view.container_handle != button.source.container_handle
                && !button
                    .source
                    .entries
                    .iter()
                    .any(|item| item.handle == view.container_handle)
        })
        .collect::<Vec<_>>();
    if destinations.is_empty() {
        return;
    }
    let Some(id) = picker.next.checked_add(1) else {
        return;
    };
    picker.next = id;
    picker.pending = Some(Selection {
        id,
        generation: owner.generation(),
        source: button.source.clone(),
        destinations,
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
    let Some(selection) = picker.pending.take() else {
        return;
    };
    if let Some(destination) = choice
        .destination
        .and_then(|index| selection.destinations.get(index))
        && let Some(play) = &owner.play
    {
        play.link
            .container_move_all_bound(&selection.source, destination);
    }
}
fn escape(
    keys: Res<ButtonInput<KeyCode>>,
    roots: Query<Entity, With<Root>>,
    guard: ModalGuard,
    mut picker: ResMut<Picker>,
) {
    if keys.just_pressed(KeyCode::Escape) && roots.iter().any(|entity| guard.allows(entity)) {
        picker.pending = None;
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut picker: ResMut<Picker>,
    roots: Query<Entity, With<Root>>,
    mut previous: Local<Option<u64>>,
) {
    if picker
        .pending
        .as_ref()
        .is_some_and(|selection| selection.generation != owner.generation() || !allowed(&owner))
    {
        picker.pending = None;
    }
    let id = picker.pending.as_ref().map(|selection| selection.id);
    if *previous == id {
        return;
    }
    *previous = id;
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
    source::dialog_caption(&mut commands, panel, "Move contents to", skins.font.clone());
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
            Some(index),
            &caption,
        );
    }
    button(&mut commands, &skins, panel, selection.id, None, "Cancel");
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    id: u64,
    destination: Option<usize>,
    caption: &str,
) {
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 240.),
            Choice { id, destination },
        ))
        .observe(choose)
        .id();
    if destination.is_some() {
        commands
            .entity(button)
            .insert(Name::new("inventory.bulk.destination"));
    }
    let caption = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
}
