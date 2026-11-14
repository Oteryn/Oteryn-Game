//! Query/status presentation over the retained world-event owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    modal::ModalGuard,
};
use crate::play::feature_0053_world_event::view::{WorldEventBinding, WorldEventControl as Kind};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::world_event::WorldEventPanel as Panel;
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Component, Clone)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, WorldEventBinding)>,
    revision: u64,
    widgets: Option<widgets::Widgets>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if owner.phase != Phase::Playing || owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    if let Some(binding) = play.link.world_event_binding() {
        let _ = play.link.world_event_dispatch_bound(&binding, Kind::Toggle);
    }
}
fn inside(mut entity: Option<Entity>, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    while let Some(current) = entity {
        if current == ancestor {
            return true;
        }
        entity = parents.get(current).ok().map(ChildOf::parent);
    }
    false
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let binding = owner
        .play
        .as_ref()
        .and_then(|play| play.link.world_event_binding())
        .filter(|b| b.panel.open);
    let Some(binding) = binding else {
        if let Some(widgets) = state.widgets.take() {
            if inside(focus.get(), widgets.root, &parents) {
                focus.clear();
            }
            commands.entity(widgets.root).despawn();
        }
        state.snapshot = None;
        return;
    };
    if state.snapshot.as_ref().is_some_and(|(key, old)| {
        key.generation == owner.generation() && old.panel == binding.panel
    }) {
        return;
    }
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    let key = Key {
        generation: owner.generation(),
        revision,
    };
    if let Some(widgets) = state.widgets.take() {
        if inside(focus.get(), widgets.root, &parents) {
            focus.clear();
        }
        commands.entity(widgets.root).despawn();
    }
    state.widgets = Some(widgets::build(&mut commands, &skins, key, &binding.panel));
    state.snapshot = Some((key, binding));
}
fn perform(owner: &HostOwner, state: &State, control: &Control) {
    let Some((key, binding)) = &state.snapshot else {
        return;
    };
    if control.key != *key
        || key.generation != owner.generation()
        || owner.modal_active()
        || owner.phase != Phase::Playing
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let _ = play.link.world_event_dispatch_bound(binding, control.kind);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &state, control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    guard: ModalGuard,
    focus: Res<InputFocus>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    parents: Query<&ChildOf>,
) {
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        if event.key_code == KeyCode::KeyN
            && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
            && !keys.any_pressed([
                KeyCode::AltLeft,
                KeyCode::AltRight,
                KeyCode::ShiftLeft,
                KeyCode::ShiftRight,
                KeyCode::SuperLeft,
                KeyCode::SuperRight,
            ])
            && guard.allows_focus(None)
            && owner
                .play
                .as_ref()
                .is_some_and(|p| !p.link.preference_chat_open())
        {
            open(&owner);
        } else if event.key_code == KeyCode::Escape
            && let Some(widgets) = &state.widgets
            && inside(focus.get(), widgets.root, &parents)
            && guard.allows(widgets.root)
            && let Some((key, _)) = &state.snapshot
        {
            perform(
                &owner,
                &state,
                &Control {
                    key: *key,
                    kind: Kind::Close,
                },
            );
        }
    }
}
