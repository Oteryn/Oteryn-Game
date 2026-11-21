//! Drop Tracker selection over confirmed loot, independent of session counters.
use super::*;
use bevy::{
    input_focus::tab_navigation::TabIndex,
    text::{EditableText, TextCursorStyle},
    ui_widgets::TextInput,
};
use oteryn_client_ui_bevy as source;
use std::collections::VecDeque;

#[derive(Component)]
struct ItemField;
#[derive(Clone, Copy)]
enum Action {
    Track,
    Untrack,
    Filter,
}
#[derive(Component, Clone, Copy)]
pub(super) struct TrackerControl {
    pub generation: u64,
    action: Action,
}
#[derive(Resource, Default)]
struct Pending(VecDeque<(Entity, TrackerControl)>);

pub(super) fn install(app: &mut App) {
    app.init_resource::<Pending>()
        .add_observer(activate)
        .add_systems(
            Update,
            finish
                .after(bevy::text::EditableTextSystems)
                .before(project),
        );
}
fn activate(event: On<Activate>, controls: Query<&TrackerControl>, mut pending: ResMut<Pending>) {
    if let Ok(control) = controls.get(event.entity) {
        pending.0.push_back((event.entity, *control));
    }
}
fn finish(
    mut pending: ResMut<Pending>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    fields: Query<&EditableText, With<ItemField>>,
) {
    let Some(play) = &owner.play else {
        pending.0.clear();
        return;
    };
    while let Some((entity, control)) = pending.0.pop_front() {
        if !guard.allows(entity) || owner.modal_active() || control.generation != owner.generation()
        {
            continue;
        }
        let panel = play.link.analyser_panel();
        if !panel.selected
            || !panel.open
            || !panel.open_tools.contains(&Tool::LootTracker)
            || play.view.gameplay_input_blocked(&play.link)
        {
            continue;
        }
        play.link.analyser_select(Tool::LootTracker.key());
        match control.action {
            Action::Filter => play.link.analyser_filter_tracked(),
            Action::Track | Action::Untrack => {
                let Ok(field) = fields.single() else { continue };
                if field.pending_paste.is_some()
                    || field
                        .pending_edits
                        .iter()
                        .any(bevy::text::TextEdit::is_destructive)
                {
                    pending.0.push_front((entity, control));
                    break;
                }
                play.link.analyser_track_item(
                    &field.value().to_string(),
                    matches!(control.action, Action::Track),
                );
            }
        }
    }
}

pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    panel: Entity,
    generation: u64,
) -> Entity {
    let label = commands
        .spawn(source::text(
            "Item number from the report",
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(panel).add_child(label);
    let mut value = EditableText::new("");
    value.max_characters = Some(10);
    let field = commands
        .spawn((
            ItemField,
            TextInput,
            value,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                height: px(25.),
                padding: px(4.).all(),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(field);
    let row = commands
        .spawn(Node {
            column_gap: px(8.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(panel).add_child(row);
    button(
        commands,
        skins,
        row,
        generation,
        Action::Track,
        "Track item",
    );
    button(
        commands,
        skins,
        row,
        generation,
        Action::Untrack,
        "Untrack item",
    );
    button(
        commands,
        skins,
        row,
        generation,
        Action::Filter,
        "Show tracked only",
    )
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    row: Entity,
    generation: u64,
    action: Action,
    caption: &str,
) -> Entity {
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 150.),
            TrackerControl { generation, action },
        ))
        .id();
    let label = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(label);
    commands.entity(row).add_child(button);
    label
}
