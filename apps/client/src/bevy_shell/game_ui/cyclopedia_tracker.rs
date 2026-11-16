//! Nonmodal Bestiary and Bosstiary tracker windows in the analysers column. The session owns
//! both trackers; command replies (383/384) and the domain-98 pushes refresh them.
#[cfg(test)]
mod tests;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameRegions,
    modal::ModalGuard,
};
use bevy::{
    input_focus::InputFocus,
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::InteractionDisabled,
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui::{
    cyclopedia_tracker::{CyclopediaTrackerPanel, Intent, tracker_rows},
    task_board::creature_name,
};
use oteryn_client_ui_bevy as source;

#[derive(Clone, PartialEq, Eq)]
enum Action {
    Track,
    Stop(String),
    Refresh,
    Close,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    boss: bool,
    action: Action,
}
#[derive(Component)]
struct KeyField(bool);
/// What a built window was made from; a change rebuilds it.
#[derive(PartialEq, Eq)]
struct View {
    rows: Vec<(String, String)>,
    status: Option<&'static str>,
    busy: bool,
}
#[derive(Resource, Default)]
struct State {
    generation: Option<u64>,
    /// The Bestiary window, then the Bosstiary window.
    shown: [Option<(View, Entity)>; 2],
    /// A pressed button, sent once the creature key editor settled this frame.
    request: Option<(bool, Action)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, project)
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
fn view(panel: &CyclopediaTrackerPanel, boss: bool) -> View {
    let rows = tracker_rows(panel, boss)
        .into_iter()
        .map(|entry| {
            (
                entry.creature_key.clone(),
                format!(
                    "{}: {}/{} kills{}",
                    creature_name(&entry.creature_key),
                    entry.kill_count,
                    entry.thresholds[2],
                    if entry.completed { " (complete)" } else { "" }
                ),
            )
        })
        .collect();
    View {
        rows,
        status: if !panel.selected {
            Some("Feature unavailable in this session")
        } else if panel.pending {
            Some("Waiting for the server")
        } else {
            panel.feedback
        },
        busy: panel.pending || !panel.selected,
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    regions: Res<GameRegions>,
    skins: Res<Skins>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(Entity, &KeyField, &EditableText)>,
    mut state: ResMut<State>,
) {
    let generation = owner.play.as_ref().map(|_| owner.generation());
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| play.link.cyclopedia_tracker_panel());
    let stale = generation != state.generation;
    if stale {
        state.generation = generation;
        state.request = None;
    }
    for boss in [false, true] {
        let shown = panel
            .as_ref()
            .filter(|panel| {
                if boss {
                    panel.bosstiary_widget_open
                } else {
                    panel.bestiary_widget_open
                }
            })
            .map(|panel| view(panel, boss));
        let slot = &mut state.shown[usize::from(boss)];
        if !stale && slot.as_ref().map(|(view, _)| view) == shown.as_ref() {
            continue;
        }
        let field = fields.iter().find(|(_, field, _)| field.0 == boss);
        let typed = field.map_or_else(String::new, |(_, _, text)| text.value().to_string());
        if let Some((_, root)) = slot.take() {
            if let Some((entity, _, _)) = field
                && focus.get() == Some(entity)
            {
                focus.clear();
            }
            commands.entity(root).despawn();
        }
        if let (Some(shown), Some(generation)) = (shown, generation) {
            let root = build(&mut commands, &skins, generation, boss, &shown, &typed);
            commands.entity(regions.analyzers).add_child(root);
            *slot = Some((shown, root));
        }
    }
}
fn build(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    boss: bool,
    view: &View,
    typed: &str,
) -> Entity {
    let root = commands
        .spawn((
            Name::new(if boss {
                "bosstiary_tracker.window"
            } else {
                "bestiary_tracker.window"
            }),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                padding: UiRect {
                    top: px(24.),
                    bottom: px(6.),
                    left: px(6.),
                    right: px(6.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    source::dialog_caption(
        commands,
        root,
        if boss {
            "Bosstiary Tracker"
        } else {
            "Bestiary Tracker"
        },
        skins.font.clone(),
    );
    let control = |action| Control {
        generation,
        boss,
        action,
    };
    if view.rows.is_empty() {
        text(commands, skins, root, "No tracked creatures");
    }
    for (key, label) in &view.rows {
        let row = commands
            .spawn(Node {
                width: percent(100.),
                column_gap: px(4.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .id();
        commands.entity(root).add_child(row);
        text(commands, skins, row, label);
        button(
            commands,
            skins,
            row,
            control(Action::Stop(key.clone())),
            "Stop tracking",
            view.busy,
        );
    }
    if let Some(status) = view.status {
        text(commands, skins, root, status);
    }
    text(commands, skins, root, "Creature key");
    let mut editable = EditableText::new(typed);
    editable.max_characters = Some(128);
    editable.visible_lines = Some(1.);
    let field = commands
        .spawn((
            KeyField(boss),
            TextInput,
            editable,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::default(),
            TextCursorStyle::default(),
            Node {
                width: percent(100.),
                min_height: px(24.),
                padding: px(4.).all(),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(field);
    let footer = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(root).add_child(footer);
    for (action, label) in [
        (Action::Track, "Track"),
        (Action::Refresh, "Refresh"),
        (Action::Close, "Close"),
    ] {
        let disabled = action != Action::Close && view.busy;
        button(commands, skins, footer, control(action), label, disabled);
    }
    root
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) {
    let row = commands
        .spawn((
            source::text(value, source::font(skins.font.clone(), false)),
            Node {
                min_width: px(0.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(row);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    control: Control,
    label: &str,
    disabled: bool,
) {
    let button = commands
        .spawn((source::button(skins.buttons.clone(), 100.), control))
        .observe(activate)
        .id();
    if disabled {
        commands.entity(button).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut state: ResMut<State>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || control.generation != owner.generation()
        || owner.modal_active()
    {
        return;
    }
    state.request = Some((control.boss, control.action.clone()));
}
fn finish(
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&KeyField, &EditableText)>,
) {
    if state.request.is_none()
        || fields.iter().any(|(_, text)| {
            text.pending_paste.is_some()
                || text
                    .pending_edits
                    .iter()
                    .any(bevy::text::TextEdit::is_destructive)
        })
    {
        return;
    }
    let Some((boss, action)) = state.request.take() else {
        return;
    };
    let Some((_, root)) = &state.shown[usize::from(boss)] else {
        return;
    };
    if state.generation != Some(owner.generation())
        || owner.phase != Phase::Playing
        || owner.modal_active()
        || !guard.allows(*root)
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match action {
        Action::Close => play.link.cyclopedia_tracker_widget_toggle(boss),
        Action::Refresh => play.link.cyclopedia_tracker_change(boss, Intent::Read, ""),
        Action::Stop(key) => play
            .link
            .cyclopedia_tracker_change(boss, Intent::Remove, &key),
        Action::Track => {
            let key = fields
                .iter()
                .find(|(field, _)| field.0 == boss)
                .map_or_else(String::new, |(_, text)| text.value().to_string());
            play.link
                .cyclopedia_tracker_change(boss, Intent::Add, key.trim());
        }
    }
}
