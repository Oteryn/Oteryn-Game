use super::*;
use bevy::{
    text::TextCursorStyle,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub overlay: Entity,
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, value: &str) {
    let child = commands
        .spawn(source::text(value, source::font(skin.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(child);
}
pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    generation: u64,
    snapshot: &ItemCatalogueSnapshot,
) -> Widgets {
    let overlay = commands
        .spawn((
            GameModal,
            GlobalZIndex(161),
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
                width: px(650.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect::all(px(18.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(7.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(
        commands,
        panel,
        "Item catalogue — Ctrl+Alt+Shift+I",
        skin.font.clone(),
    );
    for (index, (caption, value)) in [
        "Item name search",
        "Category (empty means all)",
        "Selected definition reference",
    ]
    .into_iter()
    .zip(snapshot.fields())
    .enumerate()
    {
        label(commands, skin, panel, caption);
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                EditableText::new(value),
                TextCursorStyle::default(),
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                source::frame(skin.inset.clone(), [1.; 4]),
                Node {
                    width: percent(100.),
                    min_height: px(24.),
                    ..default()
                },
            ))
            .id();
        if snapshot.panel.pending || !snapshot.panel.selected {
            commands.entity(field).insert(bevy::ui::InteractionDisabled);
        }
        commands.entity(panel).add_child(field);
    }
    let scroll = commands
        .spawn((
            ScrollArea,
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                max_height: px(350.),
                min_height: px(50.),
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(5.)),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(scroll);
    let text = commands
        .spawn((
            CatalogueText,
            source::text(&snapshot.text, source::font(skin.font.clone(), false)),
        ))
        .id();
    commands.entity(scroll).add_child(text);
    let footer = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(5.),
            row_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(footer);
    for (action, caption) in ACTIONS
        .into_iter()
        .enumerate()
        .map(|(a, c)| (Some(a), c))
        .chain([(None, "Close")])
    {
        let button = commands
            .spawn((
                Control {
                    generation,
                    expected: snapshot.clone(),
                    action,
                },
                source::button(skin.buttons.clone(), 145.),
            ))
            .observe(activate)
            .id();
        if action.is_some() && (snapshot.panel.pending || !snapshot.panel.selected) {
            commands
                .entity(button)
                .insert(bevy::ui::InteractionDisabled);
        }
        let caption = commands
            .spawn(source::button_caption(caption, skin.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(footer).add_child(button);
    }
    Widgets { overlay }
}
