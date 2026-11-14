use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::{EditableText, TextCursorStyle},
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    key: Key,
    snapshot: &Snapshot,
    state: &State,
) -> Entity {
    let root = commands
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
            ScrollArea,
            Node {
                width: px(790.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    top: px(24.),
                    ..UiRect::all(px(12.))
                },
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(
        commands,
        panel,
        "Team Finder: listings and applications",
        skins.font.clone(),
    );
    text(commands, skins, panel, panel_text(snapshot));
    if let Some(action) = &state.confirmation {
        let target = snapshot
            .teams
            .visible_names()
            .get(snapshot.teams.selected_row)
            .cloned()
            .unwrap_or_default();
        let caption = match action {
            Action::Member { cancel: true } => format!("Cancel the request for {target}?"),
            Action::Decide(4) => format!("Deny {target}?"),
            Action::Advertise(None) => "Remove your advertisement?".to_owned(),
            _ => "Confirm this action?".to_owned(),
        };
        text(commands, skins, panel, caption);
        form_button(
            commands,
            skins,
            panel,
            key,
            FormAction::Confirm,
            "Confirm",
            true,
        );
        form_button(
            commands,
            skins,
            panel,
            key,
            FormAction::Cancel,
            "Back",
            true,
        );
        return root;
    }
    let ready =
        snapshot.teams.selected && snapshot.teams.pending.is_none() && !snapshot.leader.pending;
    for (action, label) in [
        (
            Action::Browse {
                applicants: false,
                next: false,
            },
            "Join team — Browse",
        ),
        (
            Action::Browse {
                applicants: true,
                next: false,
            },
            "Assemble team — Applicants",
        ),
        (
            Action::Browse {
                applicants: snapshot.teams.applicants,
                next: true,
            },
            "Next page",
        ),
        (Action::Member { cancel: false }, "Request to join"),
        (Action::Member { cancel: true }, "Cancel request"),
        (Action::Decide(2), "Invite selected"),
        (Action::Decide(3), "Accept selected"),
        (Action::Decide(4), "Deny selected"),
        (Action::Advertise(None), "Remove advertisement"),
    ] {
        button(commands, skins, panel, key, action, label, ready);
    }
    for (index, name) in snapshot.teams.visible_names().into_iter().enumerate() {
        button(
            commands,
            skins,
            panel,
            key,
            Action::Select(index),
            &format!(
                "{} {name}",
                if snapshot.teams.selected_row == index {
                    ">"
                } else {
                    ""
                }
            ),
            ready,
        );
    }
    for (index, label) in [
        "Minimum level",
        "Maximum level",
        "Vocations mask",
        "Team slots",
        "Free slots",
        "Party members: 0 no, 1 yes",
        "Target: 1 boss, 2 hunt, 3 quest",
        "Boss / hunt kind / quest",
        "Hunt area",
        "Start time (Unix seconds; 0 now)",
        "Filter leader name",
        "Filter level (0 any)",
        "Filter vocations mask (0 any)",
        "Filter target: 0 any, 1 boss, 2 hunt, 3 quest",
        "Filter only open: 0 no, 1 yes",
    ]
    .into_iter()
    .enumerate()
    {
        text(commands, skins, panel, label);
        let mut editable = EditableText::new(state.fields.get(index).map_or("", String::as_str));
        editable.max_characters = Some(128);
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
                    height: px(25.),
                    min_height: px(25.),
                    padding: UiRect::all(px(4.)),
                    ..default()
                },
            ))
            .id();
        commands.entity(panel).add_child(field);
    }
    form_button(
        commands,
        skins,
        panel,
        key,
        FormAction::Publish,
        "Publish advertisement",
        ready && snapshot.leader.selected,
    );
    form_button(
        commands,
        skins,
        panel,
        key,
        FormAction::Filter,
        "Apply team filters",
        ready,
    );
    form_button(
        commands,
        skins,
        panel,
        key,
        FormAction::Clear,
        "Clear team filters",
        ready,
    );
    button(commands, skins, panel, key, Action::Close, "Close", true);
    root
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    action: Action,
    label: &str,
    enabled: bool,
) {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), 360.),
            Control { key, action },
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
fn form_button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    action: FormAction,
    label: &str,
    enabled: bool,
) {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), 200.),
            FormControl { key, action },
        ))
        .observe(form_activate)
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
