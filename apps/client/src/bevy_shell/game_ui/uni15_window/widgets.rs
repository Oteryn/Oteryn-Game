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
    model: &oteryn_client_ui::mail::Panel,
) -> Entity {
    let overlay = commands
        .spawn((
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
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Writing and mail", skins.font.clone());
    for (index, label) in [
        "Selected item",
        "Selected mailbox (for posting)",
        "Text (include recipient address for mail)",
    ]
    .iter()
    .enumerate()
    {
        let label = commands
            .spawn(source::text(
                *label,
                source::font(skins.font.clone(), false),
            ))
            .id();
        commands.entity(panel).add_child(label);
        let mut editable = EditableText::new(&model.fields[index]);
        editable.allow_newlines = index == 2;
        editable.visible_lines = Some(if index == 2 { 6. } else { 1. });
        editable.max_characters = Some(7996);
        let mut field = commands.spawn((
            Field(index),
            TextInput,
            editable,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::default(),
            TextCursorStyle::default(),
            TabIndex(index as i32),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                min_height: px(26.),
                padding: px(4.).all(),
                ..default()
            },
        ));
        if model.pending {
            field.insert(InteractionDisabled);
        }
        let field = field.id();
        commands.entity(panel).add_child(field);
    }
    let feedback = commands
        .spawn(source::text(
            oteryn_client_ui::mail::panel_text(model),
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(panel).add_child(feedback);
    for (label, intent) in [
        ("Write", Some(Intent::Write)),
        ("Post", Some(Intent::Post)),
        ("Close", None),
    ] {
        let mut button = commands.spawn((
            source::button(skins.buttons.clone(), 80.),
            Control { generation, intent },
        ));
        if model.pending || !model.selected {
            button.insert(InteractionDisabled);
        }
        let button = button.observe(activate).id();
        let caption = commands
            .spawn(source::button_caption(label, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(panel).add_child(button);
    }
    overlay
}
