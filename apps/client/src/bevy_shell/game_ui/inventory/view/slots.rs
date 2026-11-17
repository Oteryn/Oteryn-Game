//! Equipment and container slot grids.
use super::*;

pub(super) fn equipment_grid(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    snapshot: &Snapshot,
) {
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
pub(super) fn grid(
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

pub(super) fn slot_node(
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
        super::super::tooltips::attach(commands, skins, entity, item);
        commands
            .entity(entity)
            .insert(super::super::super::ItemDragSource {
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
        let art = super::super::super::art::fixed(commands, &skins.font, key, Vec2::splat(32.));
        commands.entity(art).insert(Pickable::IGNORE);
        commands.entity(entity).add_child(art);
    }
}

pub(super) fn container(
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
                super::super::ContainerToolLaunch {
                    generation: snapshot.generation,
                    view: view.view_id,
                    handle: view.container_handle,
                },
            ))
            .observe(super::super::launch_container_tool)
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
    super::super::bulk::header(commands, skins, row, snapshot.generation, view);
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
