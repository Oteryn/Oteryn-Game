use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui::weapon_proficiency::{mastery_target, progress_level, progress_percent};
use oteryn_client_ui_bevy as source;

pub(super) struct Widgets {
    pub overlay: Entity,
    window: Entity,
    content: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skin: &Skins) -> Widgets {
    let overlay = commands
        .spawn((
            Overlay,
            GameModal,
            GlobalZIndex(160),
            TabGroup {
                order: 3,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let window = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(1000.),
                height: px(600.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(10.),
                    right: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(window);
    source::dialog_caption(commands, window, "Weapon proficiency", skin.font.clone());
    Widgets {
        overlay,
        window,
        content: None,
    }
}
pub(super) fn render(
    commands: &mut Commands,
    skin: &Skins,
    widgets: &mut Widgets,
    key: Key,
    panel: &WeaponProficiencyPanel,
) {
    let previous = widgets.content;
    let window = commands
        .spawn(Node {
            width: percent(100.),
            flex_grow: 1.,
            min_height: px(0.),
            flex_direction: FlexDirection::Column,
            row_gap: px(8.),
            ..default()
        })
        .id();
    commands.entity(widgets.window).add_child(window);
    widgets.content = Some(window);
    let body = commands
        .spawn(Node {
            width: percent(100.),
            flex_grow: 1.,
            min_height: px(0.),
            column_gap: px(8.),
            ..default()
        })
        .id();
    commands.entity(window).add_child(body);
    let list = pane(commands, skin, body, px(280.), 0);
    text(commands, skin, list, "Weapons");
    if let Some(catalogue) = &panel.catalogue {
        for row in panel.visible_weapons() {
            let entry = &catalogue.entries[row];
            let caption = format!(
                "{}{} — Level {}\n{} / {} XP ({}%)",
                if panel.weapon == entry.definition {
                    "Selected: "
                } else {
                    ""
                },
                entry.name,
                progress_level(entry),
                entry.experience,
                mastery_target(entry),
                progress_percent(entry)
            );
            button(
                commands,
                skin,
                list,
                Control {
                    key,
                    kind: Kind::Weapon(row),
                },
                &caption,
                panel.selected && !panel.pending && entry.definition != 0,
            );
            let track = commands
                .spawn((
                    Node {
                        width: percent(100.),
                        height: px(8.),
                        flex_shrink: 0.,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
                ))
                .id();
            let fill = commands
                .spawn((
                    Progress,
                    Node {
                        width: percent(progress_percent(entry) as f32),
                        height: percent(100.),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.6, 0.3)),
                ))
                .id();
            commands.entity(track).add_child(fill);
            commands.entity(list).add_child(track);
        }
        text(
            commands,
            skin,
            list,
            &format!("{} / {} weapons", catalogue.entries.len(), catalogue.total),
        );
    } else {
        text(commands, skin, list, "No weapon catalogue received");
    }
    let tree = pane(commands, skin, body, Val::Auto, 1);
    if let Some(entry) = panel.weapon_entry() {
        text(commands, skin, tree, &entry.name);
        let grid = commands
            .spawn(Node {
                width: percent(100.),
                display: Display::Grid,
                grid_template_columns: vec![RepeatedGridTrack::flex(entry.levels.len() as u16, 1.)],
                column_gap: px(6.),
                align_items: AlignItems::Start,
                ..default()
            })
            .id();
        commands.entity(tree).add_child(grid);
        for (level, data) in entry.levels.iter().enumerate() {
            let Some(required) = entry.thresholds.get(level) else {
                continue;
            };
            let unlocked = entry.experience >= *required;
            let column = commands
                .spawn(Node {
                    min_width: px(0.),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6.),
                    ..default()
                })
                .id();
            commands.entity(grid).add_child(column);
            text(
                commands,
                skin,
                column,
                &format!(
                    "Level {}\n{} — {} XP",
                    level + 1,
                    if unlocked { "Unlocked" } else { "Locked" },
                    required
                ),
            );
            for (perk, description) in data.perks.iter().enumerate() {
                let selected = data
                    .selected
                    .is_some_and(|selected| usize::from(selected) == perk);
                let caption = format!(
                    "{}{}",
                    if selected { "Selected: " } else { "" },
                    description
                );
                button(
                    commands,
                    skin,
                    column,
                    Control {
                        key,
                        kind: Kind::Perk(level, perk),
                    },
                    &caption,
                    panel.selected
                        && !panel.pending
                        && !panel.mutation_uncertain
                        && unlocked
                        && !selected,
                );
            }
        }
    } else {
        text(commands, skin, tree, "Select a weapon to view its perks");
    }
    let mut status = Vec::new();
    if !panel.selected {
        status.push("Weapon proficiency is unavailable in this session");
    }
    if panel.pending {
        status.push("Waiting for the server…");
    }
    if let Some(feedback) = panel.feedback {
        status.push(feedback);
    }
    if let Some(result) = &panel.result {
        status.push(result);
    }
    text(commands, skin, window, &status.join("\n"));
    let footer = commands
        .spawn(Node {
            width: percent(100.),
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::flex(2, 1.)],
            column_gap: px(8.),
            ..default()
        })
        .id();
    commands.entity(window).add_child(footer);
    button(
        commands,
        skin,
        footer,
        Control {
            key,
            kind: Kind::Refresh,
        },
        "Refresh",
        panel.selected && !panel.pending,
    );
    button(
        commands,
        skin,
        footer,
        Control {
            key,
            kind: Kind::Close,
        },
        "Close",
        true,
    );
    if let Some(previous) = previous {
        // Polls update text, progress and action fences without replacing pane/control identities.
        commands.queue(move |world: &mut World| {
            update_tree(world, previous, window);
            world.despawn(window);
        });
        widgets.content = Some(previous);
    }
}
fn pane(commands: &mut Commands, skin: &Skins, parent: Entity, width: Val, role: u32) -> Entity {
    let entity = commands
        .spawn((
            source::frame(skin.inset.clone(), [1.; 4]),
            ScrollArea,
            crate::bevy_shell::game_ui::install::RememberScroll("proficiency", role),
            Node {
                width,
                min_width: px(0.),
                min_height: px(0.),
                flex_grow: if width == Val::Auto { 1. } else { 0. },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                padding: px(5.).all(),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn text(commands: &mut Commands, skin: &Skins, parent: Entity, value: &str) {
    let entity = commands
        .spawn(source::text(value, source::font(skin.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    control: Control,
    value: &str,
    enabled: bool,
) {
    let entity = commands
        .spawn((source::button(skin.buttons.clone(), 100.), control))
        .insert(Node {
            width: percent(100.),
            min_height: px(24.),
            flex_shrink: 0.,
            padding: px(4.).all(),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .observe(activate)
        .id();
    if !enabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(value, skin.font.clone()))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}

fn copy_component<T: Component + Clone>(world: &mut World, target: Entity, source: Entity) {
    if let Some(value) = world.get::<T>(source).cloned() {
        world.entity_mut(target).insert(value);
    }
}
fn control_identity(control: &Control) -> (u8, usize, usize) {
    match control.kind {
        Kind::Close => (0, 0, 0),
        Kind::Refresh => (1, 0, 0),
        Kind::Weapon(row) => (2, row, 0),
        Kind::Perk(level, perk) => (3, level, perk),
    }
}
fn same_role(world: &World, target: Entity, source: Entity) -> bool {
    world.get::<Control>(target).map(control_identity)
        == world.get::<Control>(source).map(control_identity)
        && world.get::<Text>(target).is_some() == world.get::<Text>(source).is_some()
        && world.get::<ScrollArea>(target).is_some() == world.get::<ScrollArea>(source).is_some()
}
fn update_tree(world: &mut World, target: Entity, source: Entity) {
    copy_component::<Node>(world, target, source);
    copy_component::<Text>(world, target, source);
    copy_component::<Control>(world, target, source);
    copy_component::<BackgroundColor>(world, target, source);
    if world.get::<InteractionDisabled>(source).is_some() {
        world.entity_mut(target).insert(InteractionDisabled);
    } else {
        world.entity_mut(target).remove::<InteractionDisabled>();
    }
    let before = world
        .get::<Children>(target)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    let after = world
        .get::<Children>(source)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    let mut retained = Vec::with_capacity(after.len());
    for (index, child) in after.into_iter().enumerate() {
        if let Some(old) = before
            .get(index)
            .copied()
            .filter(|old| same_role(world, *old, child))
        {
            update_tree(world, old, child);
            retained.push(old);
        } else {
            world.entity_mut(child).remove::<ChildOf>();
            retained.push(child);
        }
    }
    world.entity_mut(target).replace_children(&retained);
    for old in before {
        if !retained.contains(&old) {
            world.despawn(old);
        }
    }
}
