use super::super::{GameRegions, ItemIcon};
use super::dialogs::{button, context};
use super::*;
use bevy::ui_widgets::Button;
use oteryn_client_ui_bevy as source;

#[derive(PartialEq)]
struct Snapshot {
    generation: u64,
    inventory: oteryn_client_ui::inventory::InventoryPanel,
    containers: oteryn_client_ui::container_tree::ContainerPanel,
    slots: Vec<oteryn_client_ui::equipment::Slot>,
    movement: oteryn_client_ui::inventory::movement::Movement,
    selection: Option<Slot>,
    menu: Option<(Slot, Vec2)>,
    world_feedback: Option<String>,
    sort_available: bool,
    windows: std::collections::BTreeMap<u8, presentation::Window>,
    browse: bool,
    /// Tibia Global's Manual Sort Mode: orange container titles, and drops name their slot.
    manual_sort: bool,
}
#[derive(Component)]
pub(super) struct Projection;
#[derive(Default)]
pub(super) struct ProjectionState(Option<Snapshot>);

// The projection reads independent retained regions and slot decorations.
#[allow(clippy::too_many_arguments)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    regions: Res<GameRegions>,
    mut ui: ResMut<InventoryUi>,
    mut previous: Local<ProjectionState>,
    roots: Query<Entity, With<Projection>>,
    mut cells: Query<(&Slot, &mut BorderColor)>,
) {
    let Some(play) = &owner.play else {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        previous.0 = None;
        *ui = InventoryUi::default();
        return;
    };
    // Retain picking identities during a gesture; refresh the authoritative stack again on drop.
    if ui
        .dragged
        .is_some_and(|slot| slot.current_item(&owner).is_some())
    {
        return;
    }
    if ui
        .selection
        .is_some_and(|slot| slot.current_item(&owner).is_none())
    {
        ui.selection = None;
    }
    if ui
        .menu
        .is_some_and(|(slot, _)| slot.current_item(&owner).is_none())
    {
        ui.menu = None;
    }
    let movement = play.link.item_movement();
    if movement.prompt.is_none() && movement.dragged.is_none() {
        ui.ground_source = None;
    }
    let prompt = movement.prompt.map(|(item, destination)| Prompt {
        generation: owner.generation(),
        item,
        destination,
    });
    if ui.prompt != prompt {
        ui.prompt = prompt;
        ui.split_count = prompt.map_or(0, |prompt| prompt.item.count.min(100));
        ui.split_text = prompt.map(|prompt| prompt.item.count.min(100).to_string());
        ui.split_entry = prompt.and_then(|prompt| super::amount::source_entry(&owner, prompt));
        ui.split_feedback.clear();
        ui.pending_amount_confirm = None;
    }
    let generation = owner.generation();
    if previous.0.as_ref().map(|snapshot| snapshot.generation) != Some(generation) {
        ui.windows.clear();
    }
    let containers = play.link.container_panel();
    presentation::retain(&mut ui, &owner, containers.views.as_ref());
    let snapshot = Snapshot {
        generation: owner.generation(),
        inventory: play.link.inventory_panel(),
        containers,
        windows: ui.windows.clone(),
        browse: play.link.browse_field_panel().selected,
        slots: play.link.equipment_slots(),
        movement,
        selection: ui.selection,
        menu: ui.menu,
        world_feedback: play.link.world_context_feedback(),
        sort_available: play.link.container_sort_panel().selected,
        manual_sort: play.link.container_manual_sort_active(),
    };
    // Selection changes decoration, not the pointer identities used for double clicks.
    for (slot, mut border) in &mut cells {
        *border = BorderColor::all(
            if snapshot.selection.is_some_and(|selected| {
                selected.item.is_some()
                    && selected.item.map(|item| item.handle) == slot.item.map(|item| item.handle)
            }) {
                Color::WHITE
            } else {
                Color::NONE
            },
        );
    }
    if previous.0.as_ref().is_some_and(|old| {
        old.generation == snapshot.generation
            && old.inventory.inventory == snapshot.inventory.inventory
            && old.containers.views == snapshot.containers.views
            && old.containers.feedback == snapshot.containers.feedback
            && old.slots == snapshot.slots
            && old.movement == snapshot.movement
            && old.menu == snapshot.menu
            && old.world_feedback == snapshot.world_feedback
            && old.sort_available == snapshot.sort_available
            && old.windows == snapshot.windows
            && old.browse == snapshot.browse
            && old.manual_sort == snapshot.manual_sort
    }) {
        previous.0 = Some(snapshot);
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    let equipment = panel(&mut commands, &skins, regions.equipment, "Inventory");
    equipment_grid(&mut commands, &skins, equipment, &snapshot);
    if snapshot.browse {
        presentation::browse_entry(&mut commands, &skins, equipment, snapshot.generation);
    }
    if snapshot.inventory.inventory.is_none() {
        label(&mut commands, &skins, equipment, "Inventory unavailable");
    }
    // The initial main-backpack snapshot remains usable before a container view is opened.
    if let Some(inventory) = &snapshot.inventory.inventory {
        let open = inventory.main_backpack.is_some_and(|backpack| {
            snapshot.containers.views.as_ref().is_some_and(|views| {
                views
                    .views
                    .iter()
                    .any(|view| view.container_handle == backpack.handle)
            })
        });
        if !open && !inventory.entries.is_empty() {
            let panel = panel(&mut commands, &skins, regions.containers, "Backpack");
            grid(
                &mut commands,
                &skins,
                panel,
                snapshot.generation,
                Location::Inventory,
                &inventory.entries,
                inventory.entries.len(),
                Some(ItemMoveDestination::MainBackpack),
                snapshot.selection,
            );
        }
    }
    if let Some(views) = &snapshot.containers.views {
        for view in &views.views {
            container(&mut commands, &skins, regions.containers, view, &snapshot);
        }
    }
    if let Some(message) = snapshot.containers.feedback.or(snapshot.movement.feedback) {
        label(&mut commands, &skins, equipment, message);
    }
    if let Some(message) = &snapshot.world_feedback {
        label(&mut commands, &skins, equipment, message);
    }
    if snapshot.movement.pending {
        label(&mut commands, &skins, equipment, "Moving item…");
    }
    if let Some((slot, position)) = snapshot.menu {
        context(&mut commands, &skins, slot, position);
    }
    previous.0 = Some(snapshot);
}
fn panel(commands: &mut Commands, skins: &Skins, parent: Entity, title: &str) -> Entity {
    titled_panel(commands, skins, parent, title, source::TEXT_COLOR)
}
fn titled_panel(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    title: &str,
    color: Color,
) -> Entity {
    let entity = commands
        .spawn((
            Projection,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: percent(100.),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.,
                padding: UiRect::all(px(3.)),
                row_gap: px(3.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    let title = commands
        .spawn((
            source::text(title, source::font(skins.font.clone(), false)),
            Node {
                height: px(14.),
                ..default()
            },
        ))
        .insert(TextColor(color))
        .id();
    commands.entity(entity).add_child(title);
    entity
}
pub(super) fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: &str) {
    let entity = commands
        .spawn(source::text(text, source::font(skins.font.clone(), true)))
        .insert((
            TextLayout::default(),
            Node {
                max_width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
fn equipment_grid(commands: &mut Commands, skins: &Skins, parent: Entity, snapshot: &Snapshot) {
    let grid = commands
        .spawn(Node {
            width: px(108.),
            align_self: AlignSelf::Center,
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::px(3, 34.),
            column_gap: px(3.),
            row_gap: px(3.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(grid);
    // Necklace / head / backpack, right hand / body / left hand, ring / legs / ammunition, feet.
    for index in [
        Some(1),
        Some(0),
        Some(2),
        Some(4),
        Some(3),
        Some(5),
        Some(8),
        Some(6),
        Some(9),
        None,
        Some(7),
        None,
    ] {
        if let Some(slot) = index.and_then(|index| snapshot.slots.get(index)) {
            let location = match slot.destination {
                ItemMoveDestination::Equipment(slot) => Location::Equipment(slot),
                _ => Location::Inventory,
            };
            slot_node(
                commands,
                skins,
                grid,
                Slot {
                    generation: snapshot.generation,
                    location,
                    item: slot.item,
                    destination: slot.available.then_some(slot.destination),
                },
                snapshot.selection,
                Some(slot.label),
            );
        } else {
            let spacer = commands
                .spawn((
                    Node {
                        width: px(34.),
                        height: px(34.),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(grid).add_child(spacer);
        }
    }
}
#[allow(clippy::too_many_arguments)] // One bounded authoritative view is rendered without inventing a second model.
fn grid(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    generation: u64,
    location: Location,
    items: &[ItemEntry],
    capacity: usize,
    destination: Option<ItemMoveDestination>,
    selection: Option<Slot>,
) {
    let grid = commands
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::px(4, 34.),
            column_gap: px(3.),
            row_gap: px(3.),
            align_self: AlignSelf::Center,
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(grid);
    for index in 0..capacity {
        // A container slot names its index; the drop sends it only in Manual Sort Mode.
        let destination = match destination {
            Some(ItemMoveDestination::Container(handle, _)) => Some(
                ItemMoveDestination::Container(handle, u32::try_from(index).ok()),
            ),
            destination => destination,
        };
        slot_node(
            commands,
            skins,
            grid,
            Slot {
                generation,
                location,
                item: items.get(index).copied(),
                destination,
            },
            selection,
            None,
        );
    }
}
fn slot_node(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    slot: Slot,
    selection: Option<Slot>,
    empty: Option<&str>,
) {
    let selected = selection.is_some_and(|selected| {
        selected.item.is_some()
            && selected.item.map(|item| item.handle) == slot.item.map(|item| item.handle)
    });
    let entity = commands
        .spawn((
            slot,
            Button,
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(34.),
                height: px(34.),
                border: UiRect::all(px(1.)),
                flex_shrink: 0.,
                ..default()
            },
            BorderColor::all(if selected { Color::WHITE } else { Color::NONE }),
        ))
        .observe(actions::select)
        .observe(actions::click)
        .observe(actions::drag_start)
        .observe(actions::drag_drop)
        .id();
    commands.entity(parent).add_child(entity);
    if let Some(item) = slot.item {
        super::tooltips::attach(commands, skins, entity, item);
        commands
            .entity(entity)
            .insert(super::super::ItemDragSource {
                generation: slot.generation,
                item,
            });
        let icon = commands
            .spawn((
                ItemIcon(item),
                ImageNode {
                    color: Color::NONE,
                    ..default()
                },
                Node {
                    width: px(32.),
                    height: px(32.),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(entity).add_child(icon);
    } else if let Some(empty) = empty {
        label(commands, skins, entity, empty);
    }
}
fn container(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    view: &oteryn_session::container_tree::ContainerView,
    snapshot: &Snapshot,
) {
    let title = if let Some(page) = view.browse {
        format!("Browse Field ({}/{})", view.entries.len(), page.total)
    } else {
        let name = snapshot
            .windows
            .get(&view.view_id)
            .and_then(|window| window.name.as_deref());
        let fallback = format!("Container #{}", view.view_id + 1);
        format!(
            "{} ({}/{})",
            name.unwrap_or(&fallback),
            view.entries.len(),
            view.capacity
        )
    };
    let panel = if snapshot.manual_sort && view.browse.is_none() {
        // Tibia Global renders container names orange while Manual Sort Mode is on.
        let [r, g, b] = oteryn_client_ui::container_manual_sort::ACTIVE_TITLE_RGB;
        titled_panel(commands, skins, parent, &title, Color::srgb_u8(r, g, b))
    } else {
        panel(commands, skins, parent, &title)
    };
    let row = commands
        .spawn(Node {
            column_gap: px(3.),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(row);
    if view.parent_handle.is_some() {
        button(
            commands,
            skins,
            row,
            "Up",
            Control::Up {
                generation: snapshot.generation,
                view: view.view_id,
                handle: view.container_handle,
            },
            36.,
        );
    }
    button(
        commands,
        skins,
        row,
        "Close",
        Control::Close {
            generation: snapshot.generation,
            view: view.view_id,
            handle: view.container_handle,
        },
        42.,
    );
    if view.browse.is_none() {
        let sort = commands
            .spawn((
                source::button(skins.buttons.clone(), 45.),
                super::ContainerToolLaunch {
                    generation: snapshot.generation,
                    view: view.view_id,
                    handle: view.container_handle,
                },
            ))
            .observe(super::launch_container_tool)
            .id();
        if !snapshot.sort_available {
            commands.entity(sort).insert(bevy::ui::InteractionDisabled);
        }
        let caption = commands
            .spawn(source::button_caption("Sort", skins.font.clone()))
            .id();
        commands.entity(sort).add_child(caption);
        commands.entity(row).add_child(sort);
    }
    super::bulk::header(commands, skins, row, snapshot.generation, view);
    let body = presentation::body(
        commands,
        skins,
        panel,
        snapshot.generation,
        view,
        snapshot.windows.get(&view.view_id),
    );
    grid(
        commands,
        skins,
        body,
        snapshot.generation,
        Location::Container {
            view: view.view_id,
            handle: view.container_handle,
        },
        &view.entries,
        usize::from(view.capacity),
        Some(ItemMoveDestination::Container(view.container_handle, None)),
        snapshot.selection,
    );
}
