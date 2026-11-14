//! Player reports project the canonical native form and dispatch through its retained owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0076_player_report::view::{
    PlayerReportControl as Kind, PlayerReportSnapshot,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Field(usize);
#[derive(Component, Clone, Copy)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Action,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Owner(Kind),
    ReviewBug,
    ConfirmBug,
    CancelBug,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<PlayerReportSnapshot>,
    generation: u64,
    revision: u64,
    overlay: Option<Entity>,
    confirmation: Option<[String; 5]>,
    draft: Option<[String; 5]>,
    dirty: bool,
    pending: Option<(Entity, Control)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
    {
        play.link.player_report_open();
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
) {
    let snapshot = owner
        .play
        .as_ref()
        .map(|play| play.link.player_report_snapshot())
        .filter(|s| s.panel.open);
    let unchanged = state.generation == owner.generation()
        && state
            .snapshot
            .as_ref()
            .zip(snapshot.as_ref())
            .is_none_or(|(old, new)| old.same_session(new))
        && state.snapshot.as_ref().map(|s| &s.panel) == snapshot.as_ref().map(|s| &s.panel)
        && (state.overlay.is_some() || snapshot.is_none());
    if unchanged && !state.dirty {
        return;
    }
    if let Some(entity) = state.overlay.take() {
        commands.entity(entity).despawn();
        focus.clear();
    }
    if !unchanged {
        state.confirmation = None;
        state.draft = None;
        state.pending = None;
    }
    state.dirty = false;
    state.generation = owner.generation();
    state.revision = state.revision.wrapping_add(1);
    state.snapshot = snapshot;
    if let Some(snapshot) = &state.snapshot {
        state.overlay = Some(widgets::build(
            &mut commands,
            &skins,
            &state,
            &snapshot.panel,
        ));
    }
}
fn perform(owner: &HostOwner, state: &mut State, control: Control, fields: [String; 5]) {
    if control.generation != owner.generation()
        || control.generation != state.generation
        || control.revision != state.revision
        || owner.modal_active()
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let Some(expected) = &state.snapshot else {
        return;
    };
    let current = play.link.player_report_snapshot();
    if !expected.same_session(&current) || current.panel != expected.panel {
        return;
    }
    if control.kind != Action::Owner(Kind::Close)
        && (!expected.panel.selected || expected.panel.pending)
    {
        return;
    }
    match control.kind {
        Action::ReviewBug => {
            state.draft = Some(fields.clone());
            state.confirmation = Some(fields);
        }
        Action::CancelBug => state.confirmation = None,
        Action::ConfirmBug => {
            if state.confirmation.as_ref() != Some(&fields) {
                return;
            }
            if play
                .link
                .player_report_dispatch_bound(expected, fields, Kind::Bug)
            {
                state.confirmation = None;
            }
        }
        Action::Owner(kind) => {
            play.link
                .player_report_dispatch_bound(expected, fields, kind);
        }
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) {
        return;
    }
    if state.pending.is_some() {
        return;
    }
    if let Some(values) = field_values(&state, &fields) {
        dispatch(&owner, &mut state, *control, values);
    } else {
        state.pending = Some((event.entity, *control));
    }
}
fn field_values(state: &State, fields: &Query<(&Field, &EditableText)>) -> Option<[String; 5]> {
    let mut values = state.snapshot.as_ref()?.panel.fields.clone();
    for (field, text) in fields {
        if text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
        {
            return None;
        }
        values[field.0] = text.value().to_string();
    }
    Some(values)
}
fn dispatch(owner: &HostOwner, state: &mut State, control: Control, values: [String; 5]) {
    let confirmation = state.confirmation.clone();
    perform(owner, state, control, values);
    if state.confirmation != confirmation {
        state.dirty = true;
    }
}
fn finish(
    fields: Query<(&Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    let Some((entity, control)) = state.pending else {
        return;
    };
    if !guard.allows(entity) || owner.modal_active() {
        state.pending = None;
        return;
    }
    if let Some(values) = field_values(&state, &fields) {
        state.pending = None;
        dispatch(&owner, &mut state, control, values);
    }
}

fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for event in events
        .read()
        .filter(|e| e.state == bevy::input::ButtonState::Pressed && !e.repeat)
    {
        if event.key_code == KeyCode::F12
            && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
            && keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight])
            && !keys.any_pressed([
                KeyCode::ShiftLeft,
                KeyCode::ShiftRight,
                KeyCode::SuperLeft,
                KeyCode::SuperRight,
            ])
            && guard.allows_focus(None)
        {
            open(&mut owner);
        } else if event.key_code == KeyCode::Escape
            && state.overlay.is_some_and(|e| guard.allows(e))
            && let Some(snapshot) = &state.snapshot
        {
            let fields = snapshot.panel.fields.clone();
            let control = Control {
                generation: state.generation,
                revision: state.revision,
                kind: Action::Owner(Kind::Close),
            };
            perform(&owner, &mut state, control, fields);
        }
    }
}
