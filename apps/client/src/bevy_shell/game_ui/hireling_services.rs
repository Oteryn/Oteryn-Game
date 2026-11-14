//! Banker hireling presentation over the retained session owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use crate::play::feature_0165_hireling::view::{
    HirelingServicesAction as Action, HirelingServicesSnapshot as Snapshot,
};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::hireling::{HirelingPanel, Intent};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Request(Intent),
    Confirm(Intent),
    Cancel,
    Close,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, Snapshot)>,
    revision: u64,
    confirmation: Option<Intent>,
    dirty: bool,
    root: Option<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn toggle(owner: &HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        let expected = play.link.hireling_services_snapshot();
        let _ = play
            .link
            .hireling_services_dispatch_bound(&expected, Action::Toggle);
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let next = owner
        .play
        .as_ref()
        .map(|play| (owner.generation(), play.link.hireling_services_snapshot()));
    let next = next.filter(|(_, snapshot)| snapshot.panel.open);
    if state.snapshot == next && !state.dirty {
        return;
    }
    if state.snapshot != next {
        state.confirmation = None;
    }
    if let Some(root) = state.root.take() {
        let mut focused = focus.get();
        while let Some(entity) = focused {
            if entity == root {
                focus.clear();
                break;
            }
            focused = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(root).despawn();
    }
    state.snapshot = next;
    state.dirty = false;
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    if let Some((generation, snapshot)) = &state.snapshot {
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            *generation,
            revision,
            &snapshot.panel,
            state.confirmation,
        ));
    }
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control) {
    let Some((generation, expected)) = &state.snapshot else {
        return;
    };
    if *generation != owner.generation()
        || control.generation != *generation
        || control.revision != state.revision
        || state.dirty
        || owner.modal_active()
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match control.kind {
        Kind::Cancel => {
            state.confirmation = None;
            state.dirty = true;
        }
        Kind::Close => {
            let _ = play
                .link
                .hireling_services_dispatch_bound(expected, Action::Close);
        }
        Kind::Request(intent) if intent != Intent::Refresh => {
            if state.confirmation.is_none()
                && play
                    .link
                    .hireling_services_dispatch_bound(expected, Action::Prepare(intent))
            {
                state.confirmation = Some(intent);
                state.dirty = true;
            }
        }
        Kind::Request(intent) | Kind::Confirm(intent) => {
            let confirmed = matches!(control.kind, Kind::Confirm(_));
            if confirmed && state.confirmation != Some(intent) {
                return;
            }
            if play
                .link
                .hireling_services_dispatch_bound(expected, Action::Request { intent, confirmed })
            {
                state.confirmation = None;
                state.dirty = true;
            }
        }
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &mut state, control);
    }
}
fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    for key in keys.read() {
        if key.repeat || key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        if key.key_code == KeyCode::F8 && guard.allows_focus(None) {
            if let Some(root) = state.root
                && !guard.allows(root)
            {
                continue;
            }
            if state.confirmation.is_none() {
                toggle(&owner);
            }
        } else if key.key_code == KeyCode::Escape
            && let Some(root) = state.root
            && guard.allows(root)
            && let Some((generation, _)) = &state.snapshot
        {
            let control = Control {
                generation: *generation,
                revision: state.revision,
                kind: if state.confirmation.is_some() {
                    Kind::Cancel
                } else {
                    Kind::Close
                },
            };
            perform(&owner, &mut state, &control);
        }
    }
}
