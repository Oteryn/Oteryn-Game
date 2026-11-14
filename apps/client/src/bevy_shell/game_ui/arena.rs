//! Arena presentation over the retained capability-170 owner and shared Win32 formatter.
mod widgets;
#[cfg(test)]
mod tests;
use super::{super::{owner::HostOwner, ui::Skins}, modal::ModalGuard};
use crate::play::feature_0170_arena::view::{ArenaSnapshot, arena_needs_confirmation};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::arena::Intent;

#[derive(Component, Clone)]
struct Control {
    generation: u64,
    revision: u64,
    action: Action,
}
#[derive(Clone)]
enum Action {
    Request(Intent, String),
    Confirm,
    Cancel,
    Close,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<ArenaSnapshot>,
    generation: u64,
    revision: u64,
    root: Option<Entity>,
    confirmation: Option<(Intent, String)>,
    dirty: bool,
}
pub(in crate::bevy_shell) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn toggle(owner: &HostOwner) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        let _ = play.link.arena_toggle_bound(&play.link.arena_snapshot());
    }
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control) {
    if control.generation != owner.generation()
        || control.generation != state.generation
        || control.revision != state.revision
        || owner.modal_active()
    {
        return;
    }
    let Some(play) = &owner.play else { return; };
    let Some(snapshot) = &state.snapshot else { return; };
    if play.view.gameplay_input_blocked(&play.link)
        || play.link.arena_panel() != snapshot.panel
        || !snapshot.panel.open
    {
        return;
    }
    match &control.action {
        Action::Close => {
            let _ = play.link.arena_toggle_bound(snapshot);
        }
        Action::Cancel => {
            state.confirmation = None;
            state.dirty = true;
        }
        Action::Confirm => {
            if let Some((intent, _)) = &state.confirmation {
                let _ = play.link.arena_dispatch_bound(snapshot, *intent, true);
            }
            state.confirmation = None;
            state.dirty = true;
        }
        Action::Request(intent, label) => {
            if state.confirmation.is_some() || !snapshot.panel.selected || snapshot.panel.pending.is_some() {
                return;
            }
            if arena_needs_confirmation(*intent) {
                state.confirmation = Some((*intent, label.clone()));
                state.dirty = true;
            } else {
                let _ = play.link.arena_dispatch_bound(snapshot, *intent, false);
            }
        }
    }
    // Invalidate all controls immediately, including a second activation in this frame.
    state.revision = state.revision.saturating_add(1);
}
fn activate(
    event: On<Activate>, controls: Query<&Control>, guard: ModalGuard,
    owner: NonSend<HostOwner>, mut state: ResMut<State>,
) {
    if guard.allows(event.entity) && let Ok(control) = controls.get(event.entity) {
        perform(&owner, &mut state, control);
    }
}
fn project(mut commands: Commands, owner: NonSend<HostOwner>, skins: Res<Skins>, mut state: ResMut<State>, mut focus: ResMut<InputFocus>) {
    let snapshot = owner.play.as_ref().map(|play| play.link.arena_snapshot());
    let visible = snapshot.as_ref().is_some_and(|snapshot| snapshot.panel.open && snapshot.panel.selected);
    let changed = state.generation != owner.generation()
        || snapshot.as_ref().map(|s| &s.panel) != state.snapshot.as_ref().map(|s| &s.panel);
    if !changed && !state.dirty { return; }
    if let Some(root) = state.root.take() {
        // All window controls are rebuilt from one owner snapshot.
        focus.clear();
        commands.entity(root).despawn();
    }
    if changed { state.confirmation = None; }
    state.generation = owner.generation();
    state.revision = state.revision.saturating_add(1);
    state.snapshot = snapshot;
    state.dirty = false;
    if visible {
        state.root = Some(widgets::build(&mut commands, &skins, &state));
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keys: Option<Res<ButtonInput<KeyCode>>>, guard: ModalGuard, focus: Res<InputFocus>,
    owner: NonSend<HostOwner>, mut state: ResMut<State>,
) {
    for event in events.read().filter(|event| event.state == bevy::input::ButtonState::Pressed && !event.repeat) {
        if event.key_code == KeyCode::KeyZ
            && keys.as_ref().is_some_and(|keys|
                keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
                && keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
                && !keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight, KeyCode::SuperLeft, KeyCode::SuperRight]))
            && (guard.allows_focus(focus.get()) || state.root.is_some_and(|root| guard.allows(root)))
        {
            toggle(&owner);
        } else if event.key_code == KeyCode::Escape
            && state.root.is_some_and(|root| guard.allows(root))
        {
            let control = Control {
                generation: state.generation, revision: state.revision,
                action: if state.confirmation.is_some() { Action::Cancel } else { Action::Close },
            };
            perform(&owner, &mut state, &control);
        }
    }
}
