use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::TextInput,
};
use oteryn_client_ui_bevy as source;
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    revision: u64,
    model: &Panel,
    draft: &[String; 3],
    confirmation: Option<&Confirmation>,
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
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(520.),
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
            bevy::ui_widgets::ScrollArea,
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Guild wars", skins.font.clone());
    text(
        commands,
        skins,
        panel,
        oteryn_client_ui::guild_war::panel_text(model),
    );
    if let Some(confirmation) = confirmation {
        let action = match confirmation.intent {
            Intent::Declare => "Declare",
            Intent::Accept => "Accept",
        };
        text(
            commands,
            skins,
            panel,
            format!(
                "Confirm {action} guild war?\nTarget UUID: {}",
                confirmation.fields[0]
            ),
        );
        if confirmation.intent == Intent::Declare {
            text(
                commands,
                skins,
                panel,
                format!(
                    "Frag limit: {}\nDuration in days: {}",
                    confirmation.fields[1], confirmation.fields[2]
                ),
            );
        }
        button(
            commands,
            skins,
            panel,
            Control {
                generation,
                revision,
                kind: Kind::Confirm,
            },
            "Confirm",
            true,
        );
        button(
            commands,
            skins,
            panel,
            Control {
                generation,
                revision,
                kind: Kind::Cancel,
            },
            "Cancel",
            true,
        );
    } else {
        for (index, label) in [
            "Opponent guild UUID (declare) or war UUID (accept)",
            "Frag limit (declare)",
            "Duration in days (declare)",
        ]
        .iter()
        .enumerate()
        {
            text(commands, skins, panel, *label);
            let frame = commands
                .spawn((
                    source::frame(skins.inset.clone(), [1.; 4]),
                    Node {
                        width: percent(100.),
                        min_height: px(26.),
                        ..default()
                    },
                ))
                .id();
            let mut editable = EditableText::new(&draft[index]);
            editable.allow_newlines = false;
            editable.max_characters = Some(7996);
            editable.visible_lines = Some(1.);
            let mut field = commands.spawn((
                Field(index),
                TextInput,
                editable,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::default(),
                TextCursorStyle::default(),
                TabIndex(0),
                Node {
                    width: percent(100.),
                    min_height: px(26.),
                    padding: px(4.).all(),
                    ..default()
                },
            ));
            if model.pending || !model.selected {
                field.insert(InteractionDisabled);
            }
            let field = field.id();
            commands.entity(frame).add_child(field);
            commands.entity(panel).add_child(frame);
        }
        for (intent, caption) in [(Intent::Declare, "Declare"), (Intent::Accept, "Accept")] {
            button(
                commands,
                skins,
                panel,
                Control {
                    generation,
                    revision,
                    kind: Kind::Propose(intent),
                },
                caption,
                model.selected && !model.pending,
            );
        }
    }
    button(
        commands,
        skins,
        panel,
        Control {
            generation,
            revision,
            kind: Kind::Close,
        },
        "Close",
        true,
    );
    overlay
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    control: Control,
    label: &str,
    enabled: bool,
) {
    let mut entity = commands.spawn((source::button(skins.buttons.clone(), 100.), control));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
