use super::*;
use crate::play::feature_0287_actor_spell_catalog::view::SpellbookState;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui_widgets::{Button, ScrollArea, TextInput},
};
use oteryn_client_ui::quest_log::QuestPanel;
#[derive(Clone, Copy)]
pub(super) struct Widgets {
    pub root: Entity,
    pub search: Entity,
    pub rows: Entity,
    pub missions: Entity,
}
// This source-shaped dialog needs the three retained feature projections and its mount.
#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    overlay: Entity,
    key: Key,
    quest: &QuestPanel,
    book: &SpellbookState,
    catalogue: &oteryn_client_ui::spell_catalog::Panel,
) -> Widgets {
    let root = commands
        .spawn((
            TabGroup {
                order: 2,
                modal: true,
            },
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(785.),
                height: px(620.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(10.),
                    right: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(10.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(root);
    source::dialog_caption(
        commands,
        root,
        if key.tab == Tab::Quests {
            "Quest Log"
        } else {
            "Spellbook — Magical Archive"
        },
        skin.font.clone(),
    );
    let tabs = row(commands, root);
    action(commands, skin, tabs, key, Kind::Quests, "Quest Log", 100.);
    action(commands, skin, tabs, key, Kind::Spells, "Spells", 100.);
    let content = commands
        .spawn(Node {
            width: percent(100.),
            flex_grow: 1.,
            min_height: px(0.),
            column_gap: px(10.),
            ..default()
        })
        .id();
    commands.entity(root).add_child(content);
    let left = commands
        .spawn((
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width: percent(48.),
                min_width: px(0.),
                padding: px(5.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    let right = commands
        .spawn((
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                flex_grow: 1.,
                min_width: px(0.),
                padding: px(5.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(content).add_children(&[left, right]);
    let title = commands
        .spawn(source::text(
            if key.tab == Tab::Quests {
                "Quest Lines"
            } else {
                "Select Spell"
            },
            source::font(skin.font.clone(), false),
        ))
        .id();
    commands.entity(left).add_child(title);
    if key.tab == Tab::Spells && book.selected {
        for kind in [Kind::Vocation, Kind::Premium, Kind::Group] {
            let button = action(commands, skin, left, key, kind, "", 320.);
            let caption = commands
                .spawn((
                    source::button_caption("", skin.font.clone()),
                    FilterCaption(kind),
                ))
                .id();
            commands.entity(button).add_child(caption);
        }
        let levels = row(commands, left);
        field(
            commands,
            skin,
            levels,
            Field::Minimum,
            &book.filter.min_level.to_string(),
            10,
            70.,
        );
        field(
            commands,
            skin,
            levels,
            Field::Maximum,
            &book.filter.max_level.to_string(),
            10,
            70.,
        );
        let label = commands
            .spawn(source::text(
                "Level range (0 = any)",
                source::font(skin.font.clone(), true),
            ))
            .id();
        commands.entity(left).add_child(label);
    } else if key.tab == Tab::Spells {
        let label = commands
            .spawn(source::text(
                "Maximum level (blank = any)",
                source::font(skin.font.clone(), true),
            ))
            .id();
        commands.entity(left).add_child(label);
        field(
            commands,
            skin,
            left,
            Field::Maximum,
            &catalogue
                .max_level
                .map(|v| v.to_string())
                .unwrap_or_default(),
            10,
            120.,
        );
    }
    let rows = scroll(commands, left, true);
    let search_value = if key.tab == Tab::Quests {
        quest.list_search.as_str()
    } else if book.selected {
        book.filter.search.as_str()
    } else {
        catalogue.filter.as_str()
    };
    let search = field(
        commands,
        skin,
        left,
        Field::Search,
        search_value,
        if key.tab == Tab::Quests { 512 } else { 64 },
        320.,
    );
    let search_row = row(commands, left);
    action(commands, skin, search_row, key, Kind::Search, "Search", 80.);
    action(
        commands,
        skin,
        search_row,
        key,
        Kind::Refresh,
        "Refresh",
        80.,
    );
    if key.tab == Tab::Quests {
        let sort = action(commands, skin, left, key, Kind::Sort, "", 320.);
        let label = commands
            .spawn((
                source::button_caption("", skin.font.clone()),
                FilterCaption(Kind::Sort),
            ))
            .id();
        commands.entity(sort).add_child(label);
    }
    let missions = scroll(commands, right, false);
    if key.tab == Tab::Spells {
        commands.entity(missions).insert(Node {
            display: Display::None,
            ..default()
        });
    }
    let description = scroll(commands, right, true);
    let detail = commands
        .spawn((
            source::text("", source::font(skin.font.clone(), false)),
            Detail,
        ))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(description).add_child(detail);
    let controls = row(commands, right);
    if key.tab == Tab::Quests {
        action(
            commands,
            skin,
            controls,
            key,
            Kind::Track,
            "Track / Untrack",
            130.,
        );
    } else {
        for slot in 0..4 {
            action(
                commands,
                skin,
                controls,
                key,
                Kind::Assign(slot),
                &format!("Slot {}", slot + 1),
                64.,
            );
        }
    }
    let status = commands
        .spawn((
            source::text("", source::font(skin.font.clone(), false)),
            Status,
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(root).add_child(status);
    let footer = row(commands, root);
    action(commands, skin, footer, key, Kind::Previous, "Previous", 90.);
    action(commands, skin, footer, key, Kind::Next, "Next", 90.);
    action(commands, skin, footer, key, Kind::Close, "Close", 80.);
    Widgets {
        root,
        search,
        rows,
        missions,
    }
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let row = commands
        .spawn(Node {
            width: percent(100.),
            column_gap: px(5.),
            row_gap: px(5.),
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(row);
    row
}
fn scroll(commands: &mut Commands, parent: Entity, grow: bool) -> Entity {
    let scroll = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_grow: if grow { 1. } else { 0. },
                max_height: if grow { Val::Auto } else { px(180.) },
                flex_shrink: 1.,
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(scroll);
    scroll
}
pub(super) fn rows(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    rows: &[model::Row],
) {
    for row in rows {
        let kind = match row.id {
            model::RowId::Quest(id) => Kind::Quest(id),
            model::RowId::Mission(q, m) => Kind::Mission(q, m),
            model::RowId::Spell(id) => Kind::Spell(id),
        };
        let button = commands
            .spawn((
                Button,
                TabIndex(0),
                Action { key, kind },
                Node {
                    width: percent(100.),
                    min_height: px(if matches!(row.id, model::RowId::Spell(_)) {
                        32.
                    } else {
                        22.
                    }),
                    padding: UiRect::axes(px(4.), px(2.)),
                    flex_shrink: 0.,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(if row.selected {
                    Color::srgb_u8(88, 88, 88)
                } else {
                    Color::NONE
                }),
            ))
            .observe(activate)
            .id();
        if row.selected {
            commands.entity(button).insert(Checked);
        }
        let label = commands
            .spawn(source::text(
                &row.label,
                source::font(skin.font.clone(), false),
            ))
            .insert((
                TextLayout::default(),
                Node {
                    width: percent(100.),
                    ..default()
                },
            ))
            .id();
        commands.entity(button).add_child(label);
        commands.entity(parent).add_child(button);
    }
}
fn action(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    label: &str,
    width: f32,
) -> Entity {
    let button = commands
        .spawn((
            source::button(skin.buttons.clone(), width),
            Action { key, kind },
        ))
        .insert(Node {
            width: px(width),
            max_width: percent(100.),
            height: px(20.),
            flex_shrink: 0.,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .observe(activate)
        .id();
    if !label.is_empty() {
        let text = commands
            .spawn(source::button_caption(label, skin.font.clone()))
            .id();
        commands.entity(button).add_child(text);
    }
    commands.entity(parent).add_child(button);
    button
}
fn field(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    field: Field,
    value: &str,
    limit: usize,
    width: f32,
) -> Entity {
    let frame = commands
        .spawn((
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width: px(width),
                max_width: percent(100.),
                min_height: px(25.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    let mut editable = EditableText::new(value);
    editable.max_characters = Some(limit);
    let entity = commands
        .spawn((
            field,
            TextInput,
            editable,
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            Node {
                width: percent(100.),
                height: px(25.),
                padding: px(4.).all(),
                ..default()
            },
        ))
        .id();
    commands.entity(frame).add_child(entity);
    commands.entity(parent).add_child(frame);
    entity
}
