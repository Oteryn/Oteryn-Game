use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    state: &State,
    panel: &Panel,
) -> Entity {
    let root = commands
        .spawn((
            Name::new("cast_control.window"),
            Pickable::IGNORE,
            GlobalZIndex(166),
            TabGroup {
                order: 2,
                modal: state.confirmation.is_some(),
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .id();
    if state.confirmation.is_some() {
        commands.entity(root).insert((
            super::super::GameModal,
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ));
    }
    let body = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            ScrollArea,
            Node {
                width: px(560.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(24.),
                    bottom: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(body);
    source::dialog_caption(commands, body, "Cast settings", skins.font.clone());
    text(commands, skins, body, &presentation(panel));
    if !state.feedback.is_empty() {
        text(commands, skins, body, &state.feedback);
    }
    if let Some((action, draft)) = &state.confirmation {
        let prompt = match action {
            0 => format!(
                "Confirm start / apply: delay {} seconds, viewer limit {}, password required {}, saved access revision {}?",
                draft[0], draft[1], draft[2], draft[3]
            ),
            1 => "Stop the active cast?".into(),
            3 => "Allow this character to be shown in casts?".into(),
            _ => "Hide this character from casts?".into(),
        };
        text(commands, skins, body, &prompt);
        button(
            commands,
            skins,
            body,
            state,
            Kind::Confirm,
            "Confirm",
            panel.pending,
        );
        button(commands, skins, body, state, Kind::Cancel, "Back", false);
    } else {
        for (index, (label, value)) in [
            "Delay seconds (0–300)",
            "Viewer limit (1–50)",
            "Password required (0/1, saved in account settings)",
            "Saved access revision",
        ]
        .into_iter()
        .zip(settings(panel))
        .enumerate()
        {
            text(commands, skins, body, label);
            let mut editable = EditableText::new(value);
            editable.max_characters = Some(20);
            let field = commands
                .spawn((
                    Field(index),
                    Name::new(format!("cast_control.setting.{index}")),
                    TextInput,
                    editable,
                    source::font(skins.font.clone(), false),
                    TextColor(source::TEXT_COLOR),
                    TextLayout::no_wrap(),
                    TextCursorStyle::default(),
                    TabIndex(0),
                    source::frame(skins.inset.clone(), [1.; 4]),
                    Node {
                        width: px(200.),
                        height: px(25.),
                        padding: px(4.).all(),
                        ..default()
                    },
                ))
                .id();
            if panel.pending {
                commands.entity(field).insert(InteractionDisabled);
            }
            commands.entity(body).add_child(field);
        }
        for (action, caption) in [
            "Start / apply",
            "Stop",
            "Read consent",
            "Allow shown",
            "Hide me",
            "Refresh cast",
        ]
        .into_iter()
        .enumerate()
        {
            let disabled = panel.command(action).is_err();
            button(
                commands,
                skins,
                body,
                state,
                Kind::Request(action),
                caption,
                disabled,
            );
        }
    }
    button(commands, skins, body, state, Kind::Close, "Close", false);
    root
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) {
    let child = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(child);
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
    let child = commands
        .spawn((
            source::button(skins.buttons.clone(), 160.),
            Control {
                generation: state.generation,
                revision: state.revision,
                kind,
            },
            Name::new(format!("cast_control.{caption}")),
        ))
        .observe(activate)
        .id();
    if disabled {
        commands.entity(child).insert(InteractionDisabled);
    }
    let label = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(child).add_child(label);
    commands.entity(parent).add_child(child);
}
