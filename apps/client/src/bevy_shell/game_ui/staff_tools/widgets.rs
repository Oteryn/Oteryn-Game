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
    key: Key,
    panel: &Panel,
    confirmation: Option<bool>,
) -> Entity {
    let overlay = commands
        .spawn((
            Overlay,
            GameModal,
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
    let body = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            ScrollArea,
            Node {
                width: px(700.),
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
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(body);
    source::dialog_caption(
        commands,
        body,
        "Staff authorized actions",
        skins.font.clone(),
    );
    text(
        commands,
        skins,
        body,
        "First authorize self-inspection to activate your staff role.\nInspect, ban/unban, teleport/to player, bring, notify/broadcast, ghost/invisible, mute/unmute, kick.",
    );
    text(
        commands,
        skins,
        body,
        oteryn_client_ui::staff::panel_text(&panel.summary, &panel.feedback),
    );
    for (index, label) in [
        "Operator",
        "Authorized action (exact signed text)",
        "Authorization signature (hex)",
    ]
    .iter()
    .enumerate()
    {
        text(commands, skins, body, *label);
        if confirmation.is_some() {
            text(commands, skins, body, &panel.fields[index]);
        } else {
            let mut editable = EditableText::new(&panel.fields[index]);
            editable.allow_newlines = index == 1;
            editable.max_characters = Some(8192);
            editable.visible_lines = Some(if index == 1 { 4. } else { 1. });
            let field = commands
                .spawn((
                    Field(index),
                    TextInput,
                    editable,
                    source::frame(skins.inset.clone(), [1.; 4]),
                    source::font(skins.font.clone(), false),
                    TextColor(source::TEXT_COLOR),
                    TextLayout::default(),
                    bevy::text::TextCursorStyle::default(),
                    TabIndex(index as i32),
                    Node {
                        width: percent(100.),
                        min_height: px(28.),
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
    }
    if let Some(receipt) = confirmation {
        text(
            commands,
            skins,
            body,
            if receipt {
                "Confirm storing this exact signed receipt?"
            } else {
                "Confirm submitting this exact signed action?"
            },
        );
        button(
            commands,
            skins,
            body,
            key,
            Kind::Confirm,
            "Confirm",
            panel.pending,
        );
        button(commands, skins, body, key, Kind::Cancel, "Cancel", false);
    } else {
        button(
            commands,
            skins,
            body,
            key,
            Kind::Submit(false),
            "Submit action",
            panel.pending,
        );
        button(
            commands,
            skins,
            body,
            key,
            Kind::Submit(true),
            "Store signed receipt",
            panel.pending,
        );
    }
    button(commands, skins, body, key, Kind::Close, "Close", false);
    overlay
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let child = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(child);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    label: &str,
    disabled: bool,
) {
    let child = commands
        .spawn((
            source::button(skins.buttons.clone(), 150.),
            Control { key, kind },
        ))
        .observe(activate)
        .id();
    if disabled {
        commands.entity(child).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(child).add_child(caption);
    commands.entity(parent).add_child(child);
}
