//! Independent, nonmodal equipped-item tracker, backed by admitted inspection replies.
#[cfg(test)]
mod tests;
use super::super::{
    owner::{HostOwner, Phase},
    ui::Skins,
};
use super::modal::ModalGuard;
use bevy::{
    prelude::*,
    ui_widgets::{Activate, ScrollArea},
};
use oteryn_client_ui::imbue::tracker_text;
use oteryn_client_ui_bevy as source;

#[derive(Clone, Copy)]
enum Kind {
    Close,
    Refresh,
    Filter(&'static str),
}
#[derive(Component, Clone, Copy)]
struct Control {
    generation: u64,
    serial: u64,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    serial: u64,
    root: Option<Entity>,
    snapshot: Option<(String, String)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    let play = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing);
    if let Some(play) = play {
        play.link.imbue_tracker_poll();
    }
    let panel = play
        .map(|play| play.link.imbue_panel())
        .filter(|panel| panel.tracker_open);
    let snapshot = panel
        .as_ref()
        .map(|panel| (tracker_text(panel), panel.tracker_minutes.clone()));
    if state.generation == owner.generation() && state.snapshot == snapshot {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.generation = owner.generation();
    state.serial = state.serial.saturating_add(1);
    state.snapshot = snapshot;
    if let Some((text, filter)) = &state.snapshot {
        state.root = Some(build(
            &mut commands,
            &skins,
            state.generation,
            state.serial,
            text,
            filter,
        ));
    }
}
fn build(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    serial: u64,
    text: &str,
    filter: &str,
) -> Entity {
    let root = commands
        .spawn((
            Name::new("imbuement_tracker.window"),
            GlobalZIndex(90),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                right: px(8.),
                top: px(64.),
                width: px(280.),
                max_width: percent(95.),
                max_height: percent(55.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(8.),
                    left: px(8.),
                    right: px(8.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    source::dialog_caption(commands, root, "Imbuement Tracker", skins.font.clone());
    for (label, kind) in [
        ("All times", Kind::Filter("0")),
        ("5 minutes or less", Kind::Filter("5")),
        ("60 minutes or less", Kind::Filter("60")),
        ("Refresh equipment", Kind::Refresh),
        ("Close", Kind::Close),
    ] {
        let selected = matches!(kind, Kind::Filter(value) if value == filter || (value == "0" && filter.is_empty()));
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 260.),
                Control {
                    generation,
                    serial,
                    kind,
                },
            ))
            .observe(activate)
            .id();
        let caption = commands
            .spawn(source::button_caption(
                if selected {
                    format!("> {label}")
                } else {
                    label.into()
                },
                skins.font.clone(),
            ))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(root).add_child(button);
    }
    let body = commands
        .spawn((
            ScrollArea,
            Node {
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    let text = commands
        .spawn((
            source::text(text, source::font(skins.font.clone(), false)),
            Node {
                width: percent(100.),
                min_width: px(0.),
                ..default()
            },
        ))
        .id();
    commands.entity(body).add_child(text);
    commands.entity(root).add_child(body);
    root
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &state, control);
    }
}
fn perform(owner: &HostOwner, state: &State, control: &Control) {
    if control.generation != owner.generation()
        || control.serial != state.serial
        || owner.phase != Phase::Playing
        || owner.modal_active()
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if !play.link.imbue_panel().tracker_open || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match control.kind {
        Kind::Close => play.link.imbue_tracker_close(),
        Kind::Refresh => {
            play.link.imbue_tracker_refresh();
        }
        Kind::Filter(value) => {
            play.link.imbue_tracker_filter(value);
        }
    }
}
