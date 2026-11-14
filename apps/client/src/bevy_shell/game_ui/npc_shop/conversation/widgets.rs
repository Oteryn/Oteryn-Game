use super::*;
use bevy::{
    input_focus::tab_navigation::TabIndex,
    text::TextCursorStyle,
    ui_widgets::{ScrollArea, TextInput},
};
#[derive(Clone, Copy)]
pub(in crate::bevy_shell::game_ui::npc_shop) struct Widgets {
    pub transcript: Entity,
    pub keywords: Entity,
    pub message: Entity,
    pub controls: Entity,
    pub progress: Entity,
    pub send: Entity,
    pub trade: Entity,
}
pub(in crate::bevy_shell::game_ui::npc_shop) fn build(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    transcript_parent: Entity,
    generation: u64,
) -> Widgets {
    let transcript = commands
        .spawn(source::text("", source::font(skin.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(transcript_parent).add_child(transcript);
    let controls = commands
        .spawn(Node {
            width: percent(100.),
            flex_direction: FlexDirection::Column,
            row_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(controls);
    // NpcDialog.qml's keywordButtonSize is32+3; actual words replace unavailable button images.
    let keywords = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                height: px(35.),
                column_gap: px(5.),
                overflow: Overflow::scroll_x(),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(controls).add_child(keywords);
    let frame = commands
        .spawn((
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                height: px(25.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(controls).add_child(frame);
    let mut editor = EditableText::new("");
    editor.max_characters = Some(oteryn_client_ui::npc_dialogue::npc_service::MAX_TALK_TEXT_BYTES);
    let message = commands
        .spawn((
            Message(generation),
            TextInput,
            editor,
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
    commands.entity(frame).add_child(message);
    let actions = commands
        .spawn(Node {
            column_gap: px(5.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(controls).add_child(actions);
    let send = button(
        commands,
        skin,
        actions,
        Control {
            generation,
            target: None,
            kind: Kind::Send,
            lines: Vec::new(),
        },
        "Send",
        70.,
    );
    let trade = button(
        commands,
        skin,
        actions,
        Control {
            generation,
            target: None,
            kind: Kind::Trade,
            lines: Vec::new(),
        },
        "Trade",
        70.,
    );
    let progress = commands
        .spawn(source::text("", source::font(skin.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(progress);
    Widgets {
        transcript,
        keywords,
        message,
        controls,
        progress,
        send,
        trade,
    }
}
pub(super) fn keywords(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    generation: u64,
    actor: EntityRef,
    words: &[String],
    lines: &[String],
) {
    for word in words {
        let entity = button(
            commands,
            skin,
            parent,
            Control {
                generation,
                target: Some(actor),
                kind: Kind::Keyword(word.clone()),
                lines: lines.to_vec(),
            },
            word,
            100.,
        );
        commands.entity(entity).insert(Node {
            width: px(100.),
            height: px(35.),
            flex_shrink: 0.,
            overflow: Overflow::clip(),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        });
    }
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    control: Control,
    label: &str,
    width: f32,
) -> Entity {
    let button = commands
        .spawn((source::button(skin.buttons.clone(), width), control))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(label, skin.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
    button
}
