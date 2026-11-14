use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui::player_report::{Panel, panel_text};
use oteryn_client_ui_bevy as source;
pub(super) fn build(commands: &mut Commands, skin: &Skins, state: &State, panel: &Panel) -> Entity {
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
    let root = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            Name::new("player_report.window"),
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            ScrollArea,
            Node {
                width: px(700.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    top: px(24.),
                    left: px(12.),
                    right: px(12.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(root);
    source::dialog_caption(commands, root, "Player reports", skin.font.clone());
    text(commands, skin, root, panel_text(panel));
    text(
        commands,
        skin,
        root,
        "Ctrl+Alt+F12 opens reports; Escape closes.",
    );
    let values = state
        .confirmation
        .as_ref()
        .or(state.draft.as_ref())
        .unwrap_or(&panel.fields);
    for (index, (label, value)) in [
        "Category (bug) or reason (violation)",
        "Report type: 0 name, 1 statement, 2 bot",
        "Reported character (violation)",
        "Map bug position / optional violation translation",
        "Comment",
    ]
    .into_iter()
    .zip(values)
    .enumerate()
    {
        text(commands, skin, root, label);
        let mut editable = EditableText::new(value);
        editable.max_characters = Some(7996);
        let mut field = commands.spawn((
            Field(index),
            Name::new(format!("player_report.field.{index}")),
            TextInput,
            editable,
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::default(),
            TextCursorStyle::default(),
            TabIndex(index as i32),
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                min_height: px(26.),
                flex_shrink: 0.,
                padding: UiRect::all(px(4.)),
                ..default()
            },
        ));
        if panel.pending || !panel.selected || state.confirmation.is_some() {
            field.insert(InteractionDisabled);
        }
        let field = field.id();
        commands.entity(root).add_child(field);
    }
    if state.confirmation.is_some() {
        text(
            commands,
            skin,
            root,
            "Confirm this exact bug report. Send queues it for support.",
        );
        button(
            commands,
            skin,
            root,
            state,
            Action::ConfirmBug,
            "Confirm bug report",
            panel,
        );
        button(
            commands,
            skin,
            root,
            state,
            Action::CancelBug,
            "Back",
            panel,
        );
    } else {
        for (action, caption) in [
            (Action::ReviewBug, "Bug — review before sending"),
            (Action::Owner(Kind::ReviewViolation), "Review violation"),
            (Action::Owner(Kind::SendViolation), "Send violation"),
            (Action::Owner(Kind::ReportType(0)), "Name"),
            (Action::Owner(Kind::ReportType(1)), "Statement"),
            (Action::Owner(Kind::ReportType(2)), "Bot"),
            (Action::Owner(Kind::Reason(false)), "Previous reason"),
            (Action::Owner(Kind::Reason(true)), "Next reason"),
        ] {
            button(commands, skin, root, state, action, caption, panel);
        }
    }
    button(
        commands,
        skin,
        root,
        state,
        Action::Owner(Kind::Close),
        "Close",
        panel,
    );
    overlay
}
fn text(commands: &mut Commands, skin: &Skins, parent: Entity, value: impl Into<String>) {
    let child = commands
        .spawn(source::text(value, source::font(skin.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                max_width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(child);
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    state: &State,
    kind: Action,
    caption: &str,
    panel: &Panel,
) {
    let mut child = commands.spawn((
        source::button(skin.buttons.clone(), 260.),
        Control {
            generation: state.generation,
            revision: state.revision,
            kind,
        },
    ));
    if kind != Action::Owner(Kind::Close) && (panel.pending || !panel.selected) {
        child.insert(InteractionDisabled);
    }
    let child = child.observe(activate).id();
    let label = commands
        .spawn(source::button_caption(caption, skin.font.clone()))
        .id();
    commands.entity(child).add_child(label);
    commands.entity(parent).add_child(child);
}
