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
    cyclopedia_tracker::{CyclopediaTrackerPanel, Intent, TrackerSort, tracker_rows},
    task_board::creature_name,
};
use oteryn_client_ui_bevy as source;

#[derive(Clone, PartialEq, Eq)]
enum Action {
    Track,
    Stop(String),
    Refresh,
    Sort(TrackerSort),
    HideCompleted(bool),
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
/// What a window's rows and buttons were made from; a change rebuilds only those.
#[derive(PartialEq, Eq)]
struct View {
    rows: Vec<(String, String)>,
    status: Option<&'static str>,
    busy: bool,
    sort: TrackerSort,
    hide_completed: bool,
}
/// One open window: the persistent root, and the containers rebuilt when its view changes.
struct Shown {
    view: View,
    root: Entity,
    rows: Entity,
    footer: Entity,
}
#[derive(Resource, Default)]
struct State {
    generation: Option<u64>,
    /// The Bestiary window, then the Bosstiary window.
    shown: [Option<Shown>; 2],
    /// A pressed button, sent once the creature key editor settled this frame.
    request: Option<(bool, Action)>,
    /// The session whose saved open tracker windows were reopened.
    restored: Option<u64>,
}
/// The saved-layout panel id of a tracker window.
fn layout_id(boss: bool) -> &'static str {
    if boss {
        "bosstiary-tracker"
    } else {
        "bestiary-tracker"
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, project)
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
/// Opens or closes a tracker window and records it in the saved layout, so the next session
/// reopens it.
pub(in crate::bevy_shell) fn toggle(owner: &mut HostOwner, boss: bool) {
    let Some(play) = &owner.play else {
        return;
    };
    play.link.cyclopedia_tracker_widget_toggle(boss);
    let panel = play.link.cyclopedia_tracker_panel();
    let open = if boss {
        panel.bosstiary_widget_open
    } else {
        panel.bestiary_widget_open
    };
    if owner.settings.preferences.panel_visible(layout_id(boss)) == open {
        return;
    }
    let mut preferences = owner.settings.preferences.clone();
    if open {
        preferences.visible_panels.push(layout_id(boss).into());
    } else {
        preferences
            .visible_panels
            .retain(|id| id != layout_id(boss));
    }
    owner.settings.apply(preferences);
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
    let busy = panel.tracker_busy(boss);
    View {
        rows,
        status: if !panel.selected {
            Some("Feature unavailable in this session")
        } else if busy {
            Some("Waiting for the server")
        } else {
            panel.widget_feedback[usize::from(boss)]
        },
        busy: busy || !panel.selected,
        sort: if boss { panel.boss_sort } else { panel.sort },
        hide_completed: if boss {
            panel.boss_hide_completed
        } else {
            panel.hide_completed
        },
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    regions: Res<GameRegions>,
    skins: Res<Skins>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(Entity, &KeyField)>,
    mut state: ResMut<State>,
) {
    let generation = owner.play.as_ref().map(|_| owner.generation());
    let play = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing);
    if let Some(play) = play {
        play.link.cyclopedia_tracker_flush();
    }
    let mut panel = play.map(|play| play.link.cyclopedia_tracker_panel());
    let stale = generation != state.generation;
    if stale {
        state.generation = generation;
        state.request = None;
    }
    if let (Some(play), Some(current)) = (play, &panel)
        && current.selected
        && state.restored != generation
    {
        state.restored = generation;
        for (boss, open) in [
            (false, current.bestiary_widget_open),
            (true, current.bosstiary_widget_open),
        ] {
            if !open && owner.settings.preferences.panel_visible(layout_id(boss)) {
                play.link.cyclopedia_tracker_widget_toggle(boss);
            }
        }
        panel = Some(play.link.cyclopedia_tracker_panel());
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
        let (Some(shown), Some(generation)) = (shown, generation) else {
            if let Some(window) = slot.take() {
                clear_focus(&mut focus, &fields, boss);
                commands.entity(window.root).despawn();
            }
            continue;
        };
        if stale && let Some(window) = slot.take() {
            clear_focus(&mut focus, &fields, boss);
            commands.entity(window.root).despawn();
        }
        match slot {
            Some(window) if window.view == shown => {}
            Some(window) => {
                // Rows and buttons only: the key editor keeps its focus and pending keystrokes.
                commands.entity(window.rows).despawn_children();
                commands.entity(window.footer).despawn_children();
                fill(&mut commands, &skins, generation, boss, &shown, window);
                window.view = shown;
            }
            None => {
                let window = build(&mut commands, &skins, boss, shown);
                fill(
                    &mut commands,
                    &skins,
                    generation,
                    boss,
                    &window.view,
                    &window,
                );
                commands.entity(regions.analyzers).add_child(window.root);
                *slot = Some(window);
            }
        }
    }
}
fn clear_focus(focus: &mut InputFocus, fields: &Query<(Entity, &KeyField)>, boss: bool) {
    if let Some((entity, _)) = fields.iter().find(|(_, field)| field.0 == boss)
        && focus.get() == Some(entity)
    {
        focus.clear();
    }
}
/// The window frame, its rows and footer containers, and (Bestiary only) the creature key editor.
fn build(commands: &mut Commands, skins: &Skins, boss: bool, view: View) -> Shown {
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
    let rows = commands
        .spawn(Node {
            width: percent(100.),
            flex_direction: FlexDirection::Column,
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(root).add_child(rows);
    // Bosses are tracked from their Bosstiary entry. The Bestiary catalogue does not carry
    // the creature key yet, so the Bestiary tracker still takes it typed.
    if !boss {
        text(commands, skins, root, "Creature key");
        let mut editable = EditableText::new("");
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
    }
    let footer = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(root).add_child(footer);
    Shown {
        view,
        root,
        rows,
        footer,
    }
}
/// The tracked rows, the status line and the footer buttons of one window.
fn fill(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    boss: bool,
    view: &View,
    window: &Shown,
) {
    let control = |action| Control {
        generation,
        boss,
        action,
    };
    if view.rows.is_empty() {
        text(commands, skins, window.rows, "No tracked creatures");
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
        commands.entity(window.rows).add_child(row);
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
        text(commands, skins, window.rows, status);
    }
    let next_sort = match view.sort {
        TrackerSort::Name => TrackerSort::Kills,
        TrackerSort::Kills => TrackerSort::Remaining,
        TrackerSort::Remaining => TrackerSort::Name,
    };
    let sort_label = match view.sort {
        TrackerSort::Name => tr!("Sort: name"),
        TrackerSort::Kills => tr!("Sort: kills"),
        TrackerSort::Remaining => tr!("Sort: remaining"),
    };
    let mut actions = Vec::new();
    if !boss {
        actions.push((Action::Track, "Track", view.busy));
    }
    actions.extend([
        (Action::Refresh, "Refresh", view.busy),
        (Action::Sort(next_sort), sort_label, false),
        (
            Action::HideCompleted(!view.hide_completed),
            if view.hide_completed {
                tr!("Show completed")
            } else {
                tr!("Hide completed")
            },
            false,
        ),
        (Action::Close, "Close", false),
    ]);
    for (action, label, disabled) in actions {
        button(
            commands,
            skins,
            window.footer,
            control(action),
            label,
            disabled,
        );
    }
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
    mut owner: NonSendMut<HostOwner>,
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
    let Some(root) = state.shown[usize::from(boss)]
        .as_ref()
        .map(|window| window.root)
    else {
        return;
    };
    if state.generation != Some(owner.generation())
        || owner.phase != Phase::Playing
        || owner.modal_active()
        || !guard.allows(root)
    {
        return;
    }
    if action == Action::Close {
        toggle(&mut owner, boss);
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let panel = || play.link.cyclopedia_tracker_panel();
    match action {
        Action::Close => {}
        Action::Sort(sort) => {
            let hide = if boss {
                panel().boss_hide_completed
            } else {
                panel().hide_completed
            };
            play.link.cyclopedia_tracker_configure(boss, sort, hide);
        }
        Action::HideCompleted(hide) => {
            let sort = if boss {
                panel().boss_sort
            } else {
                panel().sort
            };
            play.link.cyclopedia_tracker_configure(boss, sort, hide);
        }
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
