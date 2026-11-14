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
    panel: &Panel,
    confirmation: Option<&Confirmation>,
) -> Entity {
    let overlay = commands
        .spawn((
            Overlay,
            super::super::GameModal,
            GlobalZIndex(166),
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
    let body = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            ScrollArea,
            Node {
                width: px(650.),
                max_width: percent(95.),
                max_height: percent(95.),
                overflow: Overflow::scroll_y(),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(24.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(body);
    source::dialog_caption(commands, body, "Boss difficulty", skins.font.clone());
    label(
        commands,
        skins,
        body,
        &oteryn_client_ui::boss_difficulty::panel_text(panel),
    );
    let context = panel
        .context
        .map(|id| {
            id.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .unwrap_or_default();
    label(
        commands,
        skins,
        body,
        &format!("Server context (automatic): {context}"),
    );
    if let Some(confirmation) = confirmation {
        let caption = if confirmation.intent == Intent::Start {
            format!(
                "Start {} at difficulty {}?",
                panel.boss_key, confirmation.difficulty
            )
        } else {
            format!(
                "Cancel the server difficulty context for {}?",
                panel.boss_key
            )
        };
        label(commands, skins, body, &caption);
        button(commands, skins, body, key, Kind::Confirm, "Confirm", true);
        button(commands, skins, body, key, Kind::Back, "Back", true);
    } else {
        label(commands, skins, body, "Boss id");
        field(commands, skins, body, Field::Boss, &panel.boss_key, 512, 0);
        label(commands, skins, body, "Difficulty");
        field(
            commands,
            skins,
            body,
            Field::Difficulty,
            &panel.difficulty.to_string(),
            2,
            1,
        );
        for (intent, caption) in [
            (Intent::Open, "Open"),
            (Intent::Preview, "Preview"),
            (Intent::Start, "Start"),
            (Intent::Cancel, "Cancel"),
        ] {
            button(
                commands,
                skins,
                body,
                key,
                Kind::Request(intent),
                caption,
                panel.selected
                    && !panel.pending
                    && (intent == Intent::Open || panel.context.is_some()),
            );
        }
    }
    button(commands, skins, body, key, Kind::Close, "Close", true);
    overlay
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(entity);
}
fn field(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    field: Field,
    value: &str,
    maximum: usize,
    index: i32,
) {
    let mut text = EditableText::new(value);
    text.max_characters = Some(maximum);
    let entity = commands
        .spawn((
            field,
            TextInput,
            text,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(index),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                height: px(25.),
                padding: px(4.).all(),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    caption: &str,
    enabled: bool,
) {
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), 100.),
        Control { key, kind },
    ));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}
