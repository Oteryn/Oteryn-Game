use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::TextInput,
};
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    state: &State,
    panel: &Panel,
) -> Entity {
    let overlay = commands
        .spawn((
            GameModal,
            GlobalZIndex(166),
            TabGroup {
                order: 3,
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
            Node {
                width: px(520.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    top: px(24.),
                    left: px(12.),
                    right: px(12.),
                    bottom: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
        ))
        .id();
    commands.entity(overlay).add_child(body);
    source::dialog_caption(commands, body, "House move-out", skins.font.clone());
    text(
        commands,
        skins,
        body,
        oteryn_client_ui::house_move_out::panel_text(panel),
    );
    if let Some((intent, fields)) = &state.confirmation {
        let message = if *intent == Intent::Notice {
            format!(
                "Confirm move-out notice for {} at {} Unix milliseconds?",
                fields[0], fields[1]
            )
        } else {
            format!("Confirm cancelling the move-out notice for {}?", fields[0])
        };
        text(commands, skins, body, message);
        button(
            commands,
            skins,
            body,
            state.revision,
            Kind::Confirm,
            "Confirm",
            true,
        );
        button(
            commands,
            skins,
            body,
            state.revision,
            Kind::Back,
            "Back",
            true,
        );
    } else {
        for (index, label) in ["House key", "Due time (Unix milliseconds)"]
            .iter()
            .enumerate()
        {
            text(commands, skins, body, *label);
            let mut editable = EditableText::new(&panel.fields[index]);
            editable.max_characters = Some(512);
            let field = commands
                .spawn((
                    Field(index),
                    TextInput,
                    editable,
                    source::font(skins.font.clone(), false),
                    TextColor(source::TEXT_COLOR),
                    TextLayout::no_wrap(),
                    TextCursorStyle::default(),
                    TabIndex(index as i32),
                    source::frame(skins.inset.clone(), [1.; 4]),
                    Node {
                        width: percent(100.),
                        height: px(28.),
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
        for (intent, label) in [
            (Intent::Query, "Query"),
            (Intent::Notice, "Notice…"),
            (Intent::Cancel, "Cancel notice…"),
        ] {
            button(
                commands,
                skins,
                body,
                state.revision,
                Kind::Request(intent),
                label,
                panel.selected && !panel.pending,
            );
        }
    }
    if !state.feedback.is_empty() {
        text(commands, skins, body, &state.feedback);
    }
    button(
        commands,
        skins,
        body,
        state.revision,
        Kind::Close,
        "Close",
        true,
    );
    overlay
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(
            value.into(),
            source::font(skins.font.clone(), false),
        ))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
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
    revision: u64,
    kind: Kind,
    label: &str,
    enabled: bool,
) {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), 140.),
            Control { revision, kind },
        ))
        .observe(activate)
        .id();
    if !enabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
