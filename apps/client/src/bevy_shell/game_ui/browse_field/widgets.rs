use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    key: Key,
    binding: &Binding,
    fields: &[String; 7],
    confirming: bool,
) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            Name::new("browse_field.window"),
            GlobalZIndex(165),
            TabGroup {
                order: 2,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(780.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(
        commands,
        panel,
        "Browse Field (Ctrl+Shift+B)",
        skins.font.clone(),
    );
    for (index, label) in [
        "Tile X",
        "Tile Y",
        "Floor",
        "View id",
        "Page index",
        "Field item row",
        "Backpack item row",
    ]
    .iter()
    .enumerate()
    {
        let row = commands
            .spawn(Node {
                column_gap: px(8.),
                ..default()
            })
            .id();
        commands.entity(panel).add_child(row);
        text(commands, skins, row, *label);
        let mut editable = EditableText::new(fields[index].clone());
        editable.max_characters = Some(12);
        let input = commands
            .spawn((
                Field(index),
                TextInput,
                editable,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::no_wrap(),
                TextCursorStyle::default(),
                TabIndex(0),
                Node {
                    width: px(125.),
                    height: px(25.),
                    padding: px(4.).all(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
            ))
            .id();
        commands.entity(row).add_child(input);
    }
    let body = commands
        .spawn((
            ScrollArea,
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                max_height: px(200.),
                min_height: px(45.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                padding: px(5.).all(),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(body);
    text(commands, skins, body, binding.text());
    if let Some(view) = binding
        .panel
        .views
        .views
        .iter()
        .find(|view| view.view_id == binding.panel.view_id)
        && let Some(page) = view.browse
    {
        text(
            commands,
            skins,
            body,
            format!(
                "Server tile: {}, {}, {}; page {}; total {}",
                page.position.x, page.position.y, page.position.floor, page.index, page.total
            ),
        );
    }
    if !binding.panel.selected {
        text(
            commands,
            skins,
            panel,
            "Browse Field is unavailable in this session",
        );
    }
    let actions = commands
        .spawn(Node {
            column_gap: px(5.),
            row_gap: px(5.),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .id();
    commands.entity(panel).add_child(actions);
    if confirming {
        text(
            commands,
            skins,
            panel,
            "Drop the selected backpack item onto this field? The server decides the result.",
        );
        button(
            commands,
            skins,
            actions,
            key,
            Kind::Confirm,
            "Confirm drop",
            binding.panel.selected && !binding.panel.pending,
        );
        button(commands, skins, actions, key, Kind::Cancel, "Cancel", true);
    } else {
        for (kind, label) in [
            (Kind::Open, "Open tile"),
            (Kind::Seek, "Seek page"),
            (Kind::Item(105), "Take to backpack"),
            (Kind::Item(106), "Drop here…"),
            (Kind::Item(107), "Open container"),
            (Kind::Item(108), "Up"),
            (Kind::Item(109), "Use field item"),
            (Kind::Item(110), "Use backpack item"),
        ] {
            button(
                commands,
                skins,
                actions,
                key,
                kind,
                label,
                binding.panel.selected && !binding.panel.pending,
            );
        }
    }
    button(commands, skins, actions, key, Kind::Close, "Close", true);
    root
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(entity);
}
// Each widget carries the exact projection key used by its observer.
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    label: &str,
    enabled: bool,
) {
    let mut button = commands.spawn((
        source::button(skins.buttons.clone(), 145.),
        Control { key, kind },
    ));
    if !enabled {
        button.insert(InteractionDisabled);
    }
    let entity = button.observe(activate).id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
