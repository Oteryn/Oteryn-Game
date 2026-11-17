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
    /// The soul points of the status widget's vitals and the inventory widget state.
    soul: Option<u32>,
    widget: super::super::hud::EquipmentWidget,
    /// The writing (115) and timed item (39) windows the item menu can open.
    write: bool,
    timed: bool,
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
    widget: Option<Res<super::super::hud::EquipmentWidget>>,
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
        soul: play.link.actor_vitals().map(|vitals| vitals.soul),
        widget: widget.as_deref().copied().unwrap_or_default(),
        write: play.link.mail_panel().selected,
        timed: play.link.timed_item_panel().selected,
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
            && old.inventory.capacity == snapshot.inventory.capacity
            && old.containers.views == snapshot.containers.views
            && old.containers.feedback == snapshot.containers.feedback
            && old.slots == snapshot.slots
            && old.movement == snapshot.movement
            && old.menu == snapshot.menu
            && old.world_feedback == snapshot.world_feedback
            && old.sort_available == snapshot.sort_available
            && old.windows == snapshot.windows
            && old.browse == snapshot.browse
            && old.soul == snapshot.soul
            && old.widget == snapshot.widget
            && old.write == snapshot.write
            && old.timed == snapshot.timed
            && old.manual_sort == snapshot.manual_sort
    }) {
        previous.0 = Some(snapshot);
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    let equipment = commands
        .spawn((
            Projection,
            Node {
                width: percent(100.),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.,
                row_gap: px(2.),
                ..default()
            },
        ))
        .id();
    commands.entity(regions.equipment).add_child(equipment);
    equipment_grid(&mut commands, &skins, equipment, &snapshot);
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
            // In Manual Sort Mode the main backpack is named by its handle, so a slot drop sends
            // the slot (capability 510); otherwise it is the main backpack as before.
            let destination = match inventory.main_backpack {
                Some(backpack) if snapshot.manual_sort => {
                    ItemMoveDestination::Container(backpack.handle, None)
                }
                _ => ItemMoveDestination::MainBackpack,
            };
            grid(
                &mut commands,
                &skins,
                panel,
                snapshot.generation,
                Location::Inventory,
                &inventory.entries,
                inventory.entries.len(),
                Some(destination),
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
        context(
            &mut commands,
            &skins,
            slot,
            position,
            (snapshot.write, snapshot.timed),
        );
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
/// Inventory.qml: the 108x142 equipment (46x42 minimised) with 32x32 slot silhouettes, the
/// 34x20 Soul and Cap boxes beside the feet and the 34x12 Blessings and Store inbox buttons.
const EQUIPMENT: Vec2 = Vec2::new(108., 142.);
const MINIMISED: Vec2 = Vec2::new(46., 42.);
const STAT_BOX: Vec2 = Vec2::new(34., 20.);
const SMALL_BUTTON: Vec2 = Vec2::new(34., 12.);
/// The silhouette of each equipment slot, in the order of the equipment slot list.
const SILHOUETTES: [&str; 10] = [
    "slot-head",
    "slot-necklace",
    "slot-backpack",
    "slot-armor",
    "slot-right-hand",
    "slot-left-hand",
    "slot-legs",
    "slot-feet",
    "slot-ring",
    "slot-ammo",
];
/// A button of the inventory widget.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum WidgetButton {
    Minimise,
    Blessings,
    Purse,
}
fn widget_button(
    event: On<bevy::ui_widgets::Activate>,
    buttons: Query<&WidgetButton>,
    guard: super::super::modal::ModalGuard,
    owner: NonSend<HostOwner>,
    widget: Option<ResMut<super::super::hud::EquipmentWidget>>,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let (Ok(button), Some(mut widget)) = (buttons.get(event.entity), widget) else {
        return;
    };
    match button {
        WidgetButton::Minimise => widget.minimised = !widget.minimised,
        WidgetButton::Blessings => widget.blessings = !widget.blessings,
        WidgetButton::Purse => {
            if let Some(play) = &owner.play {
                play.link.store_inbox_set_open(true);
            }
        }
    }
}
fn art_button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    button: WidgetButton,
    key: &'static str,
    size: Vec2,
) {
    let slot = super::super::art::fixed(commands, &skins.font, key, size);
    commands
        .entity(slot)
        .insert((button, Button))
        .observe(widget_button);
    commands.entity(parent).add_child(slot);
}
/// A Soul or Cap box: an 8 px caption over the value.
fn stat_box(commands: &mut Commands, skins: &Skins, parent: Entity, caption: &str, value: &str) {
    let entity = commands
        .spawn((
            Name::new(caption.to_owned()),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(STAT_BOX.x),
                height: px(STAT_BOX.y),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let caption = commands
        .spawn(source::text(caption, source::font(skins.font.clone(), true)))
        .id();
    let value = commands
        .spawn(source::text(value, source::font(skins.font.clone(), true)))
        .id();
    commands.entity(entity).add_children(&[caption, value]);
    commands.entity(parent).add_child(entity);
}
fn equipment_grid(commands: &mut Commands, skins: &Skins, parent: Entity, snapshot: &Snapshot) {
    let soul = snapshot
        .soul
        .map_or_else(|| "—".to_owned(), |soul| soul.to_string());
    let cap = snapshot
        .inventory
        .capacity
        .map_or_else(|| "—".to_owned(), |capacity| capacity.free_oz().to_string());
    let minimised = snapshot.widget.minimised;
    let size = if minimised { MINIMISED } else { EQUIPMENT };
    let area = commands
        .spawn((
            Name::new("Equipment"),
            Node {
                width: px(size.x),
                height: px(size.y),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(area);
    let toggle = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: px(if minimised { 0. } else { -12. }),
            top: px(0.),
            ..default()
        })
        .id();
    commands.entity(area).add_child(toggle);
    art_button(
        commands,
        skins,
        toggle,
        WidgetButton::Minimise,
        if minimised {
            "inventory-maximise"
        } else {
            "inventory-minimise"
        },
        Vec2::splat(12.),
    );
    if minimised {
        let column = commands
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2.),
                ..default()
            })
            .id();
        commands.entity(area).add_child(column);
        stat_box(commands, skins, column, "Soul", &soul);
        stat_box(commands, skins, column, "Cap", &cap);
        return;
    }
    let grid = commands
        .spawn(Node {
            width: px(EQUIPMENT.x),
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::px(3, 34.),
            column_gap: px(3.),
            row_gap: px(3.),
            align_content: AlignContent::Start,
            ..default()
        })
        .id();
    commands.entity(area).add_child(grid);
    // Necklace / head / backpack, right hand / body / left hand, ring / legs / ammunition,
    // Soul / feet / Cap.
    for (position, index) in [
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
    ]
    .into_iter()
    .enumerate()
    {
        if let Some((index, slot)) =
            index.and_then(|index| snapshot.slots.get(index).map(|slot| (index, slot)))
        {
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
                SILHOUETTES.get(index).copied(),
            );
        } else {
            let cell = commands
                .spawn((
                    Node {
                        width: px(34.),
                        height: px(34.),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::End,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(grid).add_child(cell);
            if position == 9 {
                stat_box(commands, skins, cell, "Soul", &soul);
            } else {
                stat_box(commands, skins, cell, "Cap", &cap);
            }
        }
    }
    let buttons = commands
        .spawn(Node {
            width: px(EQUIPMENT.x),
            column_gap: px(3.),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(buttons);
    art_button(
        commands,
        skins,
        buttons,
        WidgetButton::Blessings,
        "inventory-blessings",
        SMALL_BUTTON,
    );
    art_button(
        commands,
        skins,
        buttons,
        WidgetButton::Purse,
        "inventory-purse",
        SMALL_BUTTON,
    );
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
    silhouette: Option<&'static str>,
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
    } else if let Some(key) = silhouette {
        let art = super::super::art::fixed(commands, &skins.font, key, Vec2::splat(32.));
        commands.entity(art).insert(Pickable::IGNORE);
        commands.entity(entity).add_child(art);
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
    // A carried top-level container has no Up; one lying on the ground opens its tile's browse field.
    let carried = snapshot
        .inventory
        .inventory
        .iter()
        .flat_map(|inventory| {
            inventory
                .main_backpack
                .iter()
                .chain(inventory.equipment.iter().map(|equipped| &equipped.item))
        })
        .chain(snapshot.slots.iter().filter_map(|slot| slot.item.as_ref()))
        .any(|item| item.handle == view.container_handle);
    let up = if view.parent_handle.is_some() {
        Some(Control::Up {
            generation: snapshot.generation,
            view: view.view_id,
            handle: view.container_handle,
        })
    } else if snapshot.browse && view.browse.is_none() && !carried {
        Some(Control::GroundUp {
            generation: snapshot.generation,
            view: view.view_id,
            handle: view.container_handle,
        })
    } else {
        None
    };
    if let Some(up) = up {
        button(commands, skins, row, "Up", up, 36.);
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
