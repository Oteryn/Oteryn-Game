use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::TextInput,
};
use oteryn_client_ui::training::Panel;
use oteryn_client_ui_bevy as source;
pub(super) fn build(commands: &mut Commands, skin: &Skins, state: &State, panel: &Panel) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            Name::new("training.window"),
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
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(510.),
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
    commands.entity(root).add_child(body);
    source::dialog_caption(
        commands,
        body,
        "Offline / exercise training",
        skin.font.clone(),
    );
    label(
        commands,
        skin,
        body,
        oteryn_client_ui::training::panel_text(panel),
    );
    if let Some((intent, fields)) = &state.confirmation {
        let message = match intent {
            Intent::Offline => format!("Select offline statue {} and log out?", fields[0]),
            Intent::Start => format!(
                "Start exercise with weapon {} and dummy {}? This consumes weapon charges.",
                fields[1], fields[2]
            ),
            Intent::Stop => "Stop exercise training and settle consumed charges?".into(),
            Intent::Status => "Query training status?".into(),
        };
        label(commands, skin, body, message);
        button(
            commands,
            skin,
            body,
            state,
            Kind::Confirm,
            "Confirm",
            panel.selected && !panel.pending,
        );
        button(commands, skin, body, state, Kind::Cancel, "Cancel", true);
    } else {
        for (index, caption) in [
            "Offline statue handle",
            "Exercise weapon handle",
            "Exercise dummy handle",
        ]
        .into_iter()
        .enumerate()
        {
            label(commands, skin, body, caption);
            let fields = state.draft.as_ref().unwrap_or(&panel.fields);
            let mut text = EditableText::new(&fields[index]);
            text.max_characters = Some(7996);
            let mut field = commands.spawn((
                Field(index),
                Name::new(format!("training.field.{index}")),
                TextInput,
                text,
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::no_wrap(),
                TextCursorStyle::default(),
                TabIndex(index as i32),
                source::frame(skin.inset.clone(), [1.; 4]),
                Node {
                    width: percent(100.),
                    height: px(25.),
                    padding: px(4.).all(),
                    ..default()
                },
            ));
            if !panel.selected || panel.pending {
                field.insert(InteractionDisabled);
            }
            let field = field.id();
            commands.entity(body).add_child(field);
        }
        for (intent, caption) in [
            (Intent::Offline, "Offline"),
            (Intent::Start, "Start"),
            (Intent::Status, "Status"),
            (Intent::Stop, "Stop"),
        ] {
            button(
                commands,
                skin,
                body,
                state,
                Kind::Request(intent),
                caption,
                panel.selected && !panel.pending,
            );
        }
    }
    button(commands, skin, body, state, Kind::Close, "Close", true);
    root
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skin.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    state: &State,
    kind: Kind,
    caption: &str,
    enabled: bool,
) {
    let mut entity = commands.spawn((
        source::button(skin.buttons.clone(), 100.),
        Control {
            generation: state.generation,
            revision: state.revision,
            kind,
        },
    ));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let text = commands
        .spawn(source::button_caption(caption, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}
