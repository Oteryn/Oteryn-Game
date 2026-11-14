use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub overlay: Entity,
    pub panel: Entity,
    pub content: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skin: &Skins) -> Widgets {
    let overlay = commands
        .spawn((
            super::super::GameModal,
            GlobalZIndex(165),
            TabGroup {
                order: 2,
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
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(1000.),
                max_width: percent(96.),
                max_height: percent(95.),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(24.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Weapon proficiency", skin.font.clone());
    Widgets {
        overlay,
        panel,
        content: None,
    }
}
pub(super) fn render(
    commands: &mut Commands,
    skin: &Skins,
    widgets: &mut Widgets,
    key: Key,
    panel: &Panel,
    display: &str,
    confirmation: Option<usize>,
) {
    if let Some(content) = widgets.content.take() {
        commands.entity(content).despawn();
    }
    let content = column(commands, widgets.panel);
    widgets.content = Some(content);
    let body = commands
        .spawn((
            ScrollArea,
            Node {
                height: px(560.),
                max_height: percent(80.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    commands.entity(content).add_child(body);
    let enabled = panel.selected && !panel.pending && confirmation.is_none();
    if let Some(id) = confirmation {
        label(
            commands,
            skin,
            body,
            format!(
                "Confirm {} for weapon {}, perk group {}, perk {}, slot {}? Review the server costs and projection below.",
                caption(id),
                panel.weapon,
                usize::from(panel.level) + 1,
                usize::from(panel.perk) + 1,
                panel.slot
            ),
        );
        button(commands, skin, body, key, Kind::Confirm, "Confirm", true);
        button(commands, skin, body, key, Kind::Cancel, "Cancel", true);
    }
    label(commands, skin, body, "All weapons — filter by name");
    let mut edit = EditableText::new(&panel.search);
    edit.max_characters = Some(80);
    let mut field = commands.spawn((
        Search,
        TextInput,
        edit,
        source::font(skin.font.clone(), false),
        TextColor(source::TEXT_COLOR),
        TextLayout::no_wrap(),
        TextCursorStyle::default(),
        TabIndex(0),
        source::frame(skin.inset.clone(), [1.; 4]),
        Node {
            height: px(25.),
            width: percent(100.),
            padding: px(4.).all(),
            ..default()
        },
    ));
    if !enabled {
        field.insert(InteractionDisabled);
    }
    let field = field.id();
    commands.entity(body).add_child(field);
    button(
        commands,
        skin,
        body,
        key,
        Kind::Owner(Action::Search(panel.search.clone())),
        "Apply search",
        enabled,
    );
    let filters = row(commands, body);
    for (value, caption) in [
        (0, "All weapons"),
        (1, "In progress"),
        (2, "Mastered"),
        (3, "Untrained"),
    ] {
        button(
            commands,
            skin,
            filters,
            key,
            Kind::Owner(Action::Filter(value)),
            &mark(caption, panel.progress_filter == value),
            enabled,
        );
    }
    for (sort, caption) in [(false, "Name"), (true, "Progress")] {
        button(
            commands,
            skin,
            filters,
            key,
            Kind::Owner(Action::Sort(sort)),
            &mark(caption, panel.sort_progress == sort),
            enabled,
        );
    }
    for (row_index, index) in panel.visible_weapons().iter().enumerate() {
        if let Some(entry) = panel
            .catalogue
            .as_ref()
            .and_then(|page| page.entries.get(*index))
        {
            button(
                commands,
                skin,
                body,
                key,
                Kind::Owner(Action::Weapon(row_index)),
                &mark(
                    &format!(
                        "{} — {}% — {} XP",
                        entry.name,
                        oteryn_client_ui::weapon_proficiency::progress_percent(entry),
                        entry.experience
                    ),
                    panel.weapon == entry.definition,
                ),
                enabled,
            );
        }
    }
    let groups = row(commands, body);
    if let Some(entry) = panel.weapon_entry() {
        for level in 0..entry.levels.len() {
            button(
                commands,
                skin,
                groups,
                key,
                Kind::Owner(Action::Choose(1, level)),
                &mark(
                    &format!("Perk group {}", level + 1),
                    usize::from(panel.level) == level,
                ),
                enabled,
            );
        }
        if let Some(level) = entry.levels.get(usize::from(panel.level)) {
            for (perk, description) in level.perks.iter().enumerate() {
                button(
                    commands,
                    skin,
                    body,
                    key,
                    Kind::Owner(Action::Choose(2, perk)),
                    &mark(description, usize::from(panel.perk) == perk),
                    enabled,
                );
            }
        }
    }
    for (index, (text, choice)) in panel.perk_tree_rows().iter().enumerate() {
        if choice.is_some() {
            button(
                commands,
                skin,
                body,
                key,
                Kind::Owner(Action::Choose(6, index)),
                text,
                enabled,
            );
        } else {
            label(commands, skin, body, text);
        }
    }
    let slots = row(commands, body);
    for slot in 0..2 {
        let value = panel
            .shaping
            .as_ref()
            .and_then(|state| state.slots[slot].as_ref())
            .map_or_else(
                || format!("Slot {} — empty", slot + 1),
                |s| format!("Slot {} — rank {} — {}", slot + 1, s.rank, s.current),
            );
        button(
            commands,
            skin,
            slots,
            key,
            Kind::Owner(Action::Choose(3, slot)),
            &mark(&value, usize::from(panel.slot) == slot + 1),
            enabled,
        );
    }
    if let Some(slot) = panel
        .shaping
        .as_ref()
        .and_then(|state| state.slots.get(usize::from(panel.slot).wrapping_sub(1)))
        .and_then(Option::as_ref)
    {
        for (index, offer) in slot.offers.iter().enumerate() {
            button(
                commands,
                skin,
                body,
                key,
                Kind::Owner(Action::Choose(5, index)),
                &mark(offer, usize::from(panel.shaping_choice) == index),
                enabled,
            );
        }
    }
    button(
        commands,
        skin,
        body,
        key,
        Kind::Owner(Action::ClearPerk(!panel.clear_perk)),
        &mark("Clear selected perk", panel.clear_perk),
        enabled,
    );
    let actions = row(commands, body);
    for id in 101..=108 {
        button(
            commands,
            skin,
            actions,
            key,
            Kind::Review(id),
            caption(id),
            enabled,
        );
    }
    let queries = row(commands, body);
    for (id, caption) in [
        (109, "Refresh catalogue"),
        (112, "Refresh weapons"),
        (113, "Previous page"),
        (114, "Next page"),
        (116, "Refresh shaping"),
    ] {
        button(
            commands,
            skin,
            queries,
            key,
            Kind::Owner(Action::Query(id)),
            caption,
            enabled,
        );
    }
    label(commands, skin, body, display);
    button(
        commands,
        skin,
        content,
        key,
        Kind::Owner(Action::Close),
        "Close",
        true,
    );
}
pub(super) fn caption(id: usize) -> &'static str {
    match id {
        101 => "Select perk",
        102 => "Modify",
        103 => "Rank up",
        104 => "Orb rank",
        105 => "Reshape offer",
        106 => "Choose reshape",
        107 => "Decline offer",
        108 => "Clear modification",
        _ => "Unavailable action",
    }
}
pub(super) fn hud(
    commands: &mut Commands,
    skin: &Skins,
    key: Key,
    panel: &Panel,
) -> Option<Entity> {
    let hud = oteryn_client_ui::weapon_proficiency::hud_view(panel)?;
    let root = commands
        .spawn((
            Name::new("weapon_proficiency.hud"),
            GlobalZIndex(11),
            Node {
                position_type: PositionType::Absolute,
                left: px(8.),
                bottom: px(160.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            source::frame(skin.inset.clone(), [1.; 4]),
        ))
        .id();
    button(
        commands,
        skin,
        root,
        key,
        Kind::Owner(Action::Track(false)),
        &format!("{} — {}%", hud.name, hud.percent),
        !hud.pending,
    );
    if let Some((_, name)) = hud.advancement {
        button(
            commands,
            skin,
            root,
            key,
            Kind::Owner(Action::Track(true)),
            &format!("Advancement: {name}"),
            !hud.pending,
        );
    }
    Some(root)
}
fn mark(value: &str, selected: bool) -> String {
    format!("{} {value}", if selected { "☑" } else { "☐" })
}
fn column(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(4.),
            min_height: px(0.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(3.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skin.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                max_width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
#[allow(clippy::too_many_arguments)] // Every widget carries its exact owner snapshot key.
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    caption: &str,
    enabled: bool,
) {
    let mut entity = commands.spawn((
        source::button(skin.buttons.clone(), 180.),
        Control { key, kind },
    ));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let text = commands
        .spawn(source::button_caption(caption, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}
