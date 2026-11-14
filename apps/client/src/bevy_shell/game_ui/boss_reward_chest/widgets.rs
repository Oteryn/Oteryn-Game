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
    panel: &Panel,
) -> Entity {
    let root = commands
        .spawn((
            Name::new("boss_reward_chest.window"),
            crate::bevy_shell::window_drag::DraggableDialog,
            GlobalZIndex(65),
            TabGroup {
                order: 3,
                modal: false,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                right: px(20.),
                top: px(60.),
                width: px(650.),
                max_width: percent(95.),
                max_height: percent(90.),
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
    let body = root;
    source::dialog_caption(commands, body, "Boss reward chest", skins.font.clone());
    label(commands, skins, body, "Death id (32 hex digits)");
    input(commands, skins, body, Field::Death, &state.death, 32);
    label(commands, skins, body, "Reward ordinal");
    input(commands, skins, body, Field::Ordinal, &state.ordinal, 4);
    let description = commands
        .spawn((
            ScrollArea,
            Node {
                height: px(240.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(body).add_child(description);
    label(
        commands,
        skins,
        description,
        oteryn_client_ui::boss_rewards::panel_text(panel),
    );
    if panel.pending {
        label(commands, skins, body, "Waiting for the server");
    }
    if !panel.selected {
        label(
            commands,
            skins,
            body,
            "Boss rewards are unavailable in this session",
        );
    }
    for (intent, caption) in [
        (Intent::Browse, "Browse"),
        (Intent::Take, "Take item"),
        (Intent::CollectGroup, "Collect group"),
        (Intent::CollectAll, "Collect all"),
    ] {
        button(
            commands,
            skins,
            body,
            state,
            Kind::Request(intent),
            caption,
            panel.selected && !panel.pending,
        );
    }
    if let Some((intent, death, ordinal)) = &state.confirmation {
        let text = match intent {
            Intent::Take => format!("Take reward #{ordinal} from death {death}?"),
            Intent::CollectGroup => format!("Collect all rewards from death {death}?"),
            Intent::CollectAll => "Collect all live boss rewards?".into(),
            Intent::Browse => "Browse rewards?".into(),
        };
        label(commands, skins, body, text);
        button(commands, skins, body, state, Kind::Confirm, "Confirm", true);
        button(commands, skins, body, state, Kind::Cancel, "Cancel", true);
    }
    if !state.feedback.is_empty() {
        label(commands, skins, body, &state.feedback);
    }
    button(commands, skins, body, state, Kind::Close, "Close", true);
    root
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let child = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(child);
}
fn input(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    field: Field,
    value: &str,
    limit: usize,
) {
    let mut text = EditableText::new(value);
    text.max_characters = Some(limit);
    let child = commands
        .spawn((
            field,
            TextInput,
            text,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            Node {
                width: percent(100.),
                height: px(24.),
                padding: UiRect::all(px(3.)),
                ..default()
            },
            source::frame(skins.inset.clone(), [1.; 4]),
        ))
        .id();
    commands.entity(parent).add_child(child);
}
// Every button carries the current snapshot revision and uses the shared skin.
#[allow(clippy::too_many_arguments)]
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    state: &State,
    kind: Kind,
    caption: &str,
    enabled: bool,
) {
    let mut button = commands.spawn((
        source::button(skins.buttons.clone(), 135.),
        Control {
            revision: state.revision,
            kind,
        },
    ));
    if !enabled {
        button.insert(InteractionDisabled);
    }
    let button = button.observe(activate).id();
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(text);
    commands.entity(parent).add_child(button);
}
