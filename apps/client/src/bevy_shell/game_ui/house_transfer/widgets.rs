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
    key: Key,
    model: &Panel,
    confirmation: Option<&(Intent, String)>,
    draft: Option<&str>,
) -> Entity {
    let root = commands
        .spawn((
            Overlay,
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
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(480.),
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
    commands.entity(root).add_child(panel);
    source::dialog_caption(commands, panel, "House transfer", skins.font.clone());
    if let Some((intent, house)) = confirmation {
        let action = match intent {
            Intent::Enter => "Enter",
            Intent::Exit => "Exit",
        };
        text(
            commands,
            skins,
            panel,
            &format!("Confirm {action} for {house}?"),
        );
    } else {
        text(commands, skins, panel, "House key");
        let mut editable = EditableText::new(draft.unwrap_or(&model.fields[0]));
        editable.max_characters = Some(512);
        let mut field = commands.spawn((
            HouseKey,
            TextInput,
            editable,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
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
    text(
        commands,
        skins,
        panel,
        &oteryn_client_ui::house_transfer::panel_text(model),
    );
    let row = commands
        .spawn(Node {
            column_gap: px(8.),
            justify_content: JustifyContent::End,
            ..default()
        })
        .id();
    commands.entity(panel).add_child(row);
    let actions = if confirmation.is_some() {
        [
            (Kind::Confirm, "Confirm"),
            (Kind::Cancel, "Cancel"),
            (Kind::Close, "Close"),
        ]
    } else {
        [
            (Kind::Request(Intent::Enter), "Enter"),
            (Kind::Request(Intent::Exit), "Exit"),
            (Kind::Close, "Close"),
        ]
    };
    for (kind, label) in actions {
        let mut button = commands.spawn((
            source::button(skins.buttons.clone(), 80.),
            Control { key, kind },
        ));
        if kind != Kind::Close && (!model.selected || model.pending) {
            button.insert(InteractionDisabled);
        }
        let button = button.observe(activate).id();
        let caption = commands
            .spawn(source::button_caption(label, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(row).add_child(button);
    }
    root
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                max_width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
