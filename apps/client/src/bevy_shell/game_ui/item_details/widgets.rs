use super::*;
use bevy::{
    text::{EditableText, FontHinting, LineHeight, TextReadWriteMode},
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub overlay: Entity,
    pub panel: Entity,
    pub(super) content: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skin: &Skins) -> Widgets {
    let overlay = commands
        .spawn((
            Overlay,
            GameModal,
            GlobalZIndex(160),
            bevy::input_focus::tab_navigation::TabGroup {
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
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(482.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    left: px(16.),
                    right: px(16.),
                    top: px(22.),
                    bottom: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Inspect Object", skin.font.clone());
    Widgets {
        overlay,
        panel,
        content: None,
    }
}
fn text(commands: &mut Commands, skin: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skin.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(entity);
}
pub(super) fn render(
    commands: &mut Commands,
    skin: &Skins,
    widgets: &mut Widgets,
    key: Key,
    model: &Panel,
    selection: bool,
) {
    if let Some(content) = widgets.content.take() {
        commands.entity(content).despawn();
    }
    let content = commands
        .spawn(Node {
            width: percent(100.),
            min_height: px(0.),
            flex_direction: FlexDirection::Column,
            row_gap: px(8.),
            ..default()
        })
        .id();
    widgets.content = Some(content);
    commands.entity(widgets.panel).add_child(content);
    let scroll = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                max_height: px(460.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(content).add_child(scroll);
    if let Some(reply) = model.reply.as_ref().filter(|reply| reply.status == 1) {
        for item in &reply.items {
            let header = commands
                .spawn(Node {
                    column_gap: px(8.),
                    align_items: AlignItems::End,
                    ..default()
                })
                .id();
            commands.entity(scroll).add_child(header);
            let icon = commands
                .spawn((
                    source::frame(skin.inset.clone(), [1.; 4]),
                    Node {
                        width: px(66.),
                        height: px(66.),
                        flex_shrink: 0.,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .id();
            commands.entity(header).add_child(icon);
            text(commands, skin, icon, "?");
            text(commands, skin, header, format!("Inspecting {}", item.name));
            let chrome = commands
                .spawn((
                    ScrollArea,
                    source::frame(skin.inset.clone(), [1.; 4]),
                    Node {
                        width: percent(100.),
                        height: px(150.),
                        flex_shrink: 0.,
                        overflow: Overflow::scroll_y(),
                        padding: UiRect::all(px(5.)),
                        ..default()
                    },
                ))
                .id();
            commands.entity(scroll).add_child(chrome);
            let mut editor = EditableText::new(item_text(item));
            editor.allow_newlines = true;
            let details = commands
                .spawn((
                    TextInput,
                    editor,
                    TextReadWriteMode::ReadOnly,
                    FontHinting::Enabled,
                    LineHeight::Px(13.),
                    source::font(skin.font.clone(), false),
                    TextColor(source::TEXT_COLOR),
                    TextLayout::default(),
                    Node {
                        width: percent(100.),
                        min_height: px(140.),
                        flex_shrink: 0.,
                        ..default()
                    },
                ))
                .id();
            commands.entity(chrome).add_child(details);
        }
    }
    if !model.selected {
        text(
            commands,
            skin,
            scroll,
            "Item inspection unavailable for this session",
        );
    } else if model
        .reply
        .as_ref()
        .is_none_or(|reply| reply.items.is_empty())
        && !model.pending
        && model.feedback.is_empty()
    {
        text(commands, skin, scroll, "Select an item to inspect");
    }
    if !model.feedback.is_empty() {
        text(commands, skin, content, &model.feedback);
    }
    let status = commands
        .spawn((
            ClipboardStatus,
            source::text("", source::font(skin.font.clone(), false)),
        ))
        .id();
    commands.entity(content).add_child(status);
    let footer = commands
        .spawn(Node {
            column_gap: px(5.),
            justify_content: JustifyContent::End,
            flex_wrap: FlexWrap::Wrap,
            row_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(content).add_child(footer);
    for (kind, label, enabled, width) in [
        (
            Kind::Inspect,
            "Inspect selected",
            model.selected && !model.pending && selection,
            100.,
        ),
        (
            Kind::Refresh,
            "Refresh",
            model.selected && !model.pending && model.last_request.is_some_and(read_request),
            72.,
        ),
        (Kind::Copy, "Copy", copy_text(model).is_some(), 60.),
        (Kind::Close, "Close", true, 60.),
    ] {
        let button = commands
            .spawn((
                Control { key, kind },
                source::button(skin.buttons.clone(), width),
            ))
            .observe(activate)
            .id();
        if !enabled {
            commands
                .entity(button)
                .insert(bevy::ui::InteractionDisabled);
        }
        let label = commands
            .spawn(source::button_caption(label, skin.font.clone()))
            .id();
        commands.entity(button).add_child(label);
        commands.entity(footer).add_child(button);
    }
}
