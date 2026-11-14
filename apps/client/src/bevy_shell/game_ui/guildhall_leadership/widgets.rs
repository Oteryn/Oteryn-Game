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
    skin: &Skins,
    model: &Panel,
    generation: u64,
    revision: u64,
    confirmation: Option<&[String; 3]>,
) -> Entity {
    let root = commands
        .spawn((
            Overlay,
            GameModal,
            GlobalZIndex(170),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.55)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            TabGroup {
                order: 5,
                modal: true,
            },
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(520.),
                max_width: percent(96.),
                max_height: percent(94.),
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
    source::dialog_caption(commands, panel, "Guildhall leadership", skin.font.clone());
    label(
        commands,
        skin,
        panel,
        &oteryn_client_ui::guildhall::panel_text(model),
    );
    let labels = [
        "Guild UUID",
        "Successor character UUID",
        "Expected guild revision",
    ];
    for (index, caption) in labels.iter().enumerate() {
        label(commands, skin, panel, caption);
        if let Some(values) = confirmation {
            label(commands, skin, panel, &values[index]);
        } else {
            let mut editable = EditableText::new(model.fields[index].clone());
            editable.allow_newlines = false;
            editable.max_characters = Some(512);
            editable.visible_lines = Some(1.);
            let field = commands
                .spawn((
                    Field(index),
                    TextInput,
                    editable,
                    source::font(skin.font.clone(), false),
                    TextColor(source::TEXT_COLOR),
                    TextLayout::default(),
                    TextCursorStyle::default(),
                    TabIndex(index as i32),
                    Node {
                        width: percent(100.),
                        min_height: px(26.),
                        padding: px(4.).all(),
                        ..default()
                    },
                    source::frame(skin.inset.clone(), [1.; 4]),
                ))
                .id();
            if model.pending || !model.selected {
                commands.entity(field).insert(InteractionDisabled);
            }
            commands.entity(panel).add_child(field);
        }
    }
    let actions: &[(Kind, &str)] = if confirmation.is_some() {
        label(
            commands,
            skin,
            panel,
            "Transfer guildhall leadership to this successor?",
        );
        &[
            (Kind::Confirm, "Confirm transfer"),
            (Kind::Cancel, "Cancel"),
        ]
    } else {
        &[(Kind::Transfer, "Transfer"), (Kind::Close, "Close")]
    };
    for &(kind, caption) in actions {
        let button = commands
            .spawn((
                source::button(skin.buttons.clone(), 140.),
                Control {
                    generation,
                    revision,
                    kind,
                },
            ))
            .observe(super::activate)
            .id();
        let text = commands
            .spawn(source::button_caption(caption, skin.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        if kind != Kind::Close && (model.pending || !model.selected) {
            commands.entity(button).insert(InteractionDisabled);
        }
        commands.entity(panel).add_child(button);
    }
    root
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, value: &str) {
    let text = commands
        .spawn(source::text(value, source::font(skin.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(text);
}
