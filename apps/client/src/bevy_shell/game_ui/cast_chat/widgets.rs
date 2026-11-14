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
    state: &State,
    bound: &CastChatBound,
    focused: Option<Field>,
) -> (Entity, Option<Entity>) {
    let mut next_focus = None;
    let overlay = commands
        .spawn((
            Overlay,
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
    if state.confirmation.is_some() {
        commands.entity(overlay).insert(super::super::GameModal);
    }
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(680.),
                max_width: percent(95.),
                max_height: percent(95.),
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
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Live cast chat", skins.font.clone());
    let scroll = commands
        .spawn((
            ScrollArea,
            Node {
                max_height: px(320.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(scroll);
    label(
        commands,
        skins,
        scroll,
        oteryn_client_ui::cast_chat::panel_text(&bound.panel),
    );
    if let Some(confirm) = &state.confirmation {
        if state.refused {
            label(
                commands,
                skins,
                panel,
                "Cast action refused. Review the request or cancel.",
            );
        }
        label(
            commands,
            skins,
            panel,
            format!(
                "Confirm {} viewer {}?",
                action_label(confirm.action),
                confirm.subject
            ),
        );
        button(
            commands,
            skins,
            panel,
            state,
            Kind::Confirm,
            "Confirm",
            false,
        );
        button(commands, skins, panel, state, Kind::Cancel, "Cancel", false);
    } else {
        let reply = field(
            commands,
            skins,
            panel,
            Field::Reply,
            "Reply to cast room",
            &state.reply,
            256,
        );
        let subject = field(
            commands,
            skins,
            panel,
            Field::Subject,
            "Viewer handle (32 hex digits, from the live lines)",
            &state.subject,
            32,
        );
        next_focus = focused.map(|field| match field {
            Field::Reply => reply,
            Field::Subject => subject,
        });
        for action in 0..=4 {
            button(
                commands,
                skins,
                panel,
                state,
                Kind::Send(action),
                action_label(action),
                bound.panel.pending || !bound.panel.selected,
            );
        }
    }
    button(commands, skins, panel, state, Kind::Close, "Close", false);
    (overlay, next_focus)
}
fn action_label(action: u8) -> &'static str {
    match action {
        0 => "Reply",
        1 => "Mute",
        2 => "Unmute",
        3 => "Kick",
        _ => "Ban",
    }
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: impl Into<String>) {
    let child = commands
        .spawn((
            Text::new(text),
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
        ))
        .id();
    commands.entity(parent).add_child(child);
}
fn field(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    kind: Field,
    caption: &str,
    value: &str,
    limit: usize,
) -> Entity {
    label(commands, skins, parent, caption);
    let mut text = EditableText::new(value);
    text.max_characters = Some(limit);
    let child = commands
        .spawn((
            kind,
            TextInput,
            text,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            Node {
                width: percent(100.),
                height: px(25.),
                flex_shrink: 0.,
                padding: px(4.).all(),
                ..default()
            },
            source::frame(skins.inset.clone(), [1.; 4]),
        ))
        .id();
    commands.entity(parent).add_child(child);
    child
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    state: &State,
    kind: Kind,
    caption: &str,
    disabled: bool,
) {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), 100.),
            Control {
                revision: state.revision,
                kind,
            },
        ))
        .observe(activate)
        .id();
    if disabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}
