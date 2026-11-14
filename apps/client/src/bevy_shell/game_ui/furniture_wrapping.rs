//! Modeless furniture catalogue and selected action over the retained Win32 owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use crate::play::feature_0207_house_furniture_wrap::view::{
    FurnitureWrappingAction as Action, FurnitureWrappingSnapshot as Snapshot,
};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::house_furniture_wrap::{Intent, Panel};
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Close,
    Select(usize),
    Request(Intent),
}
#[derive(Component, Clone, Copy)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, Snapshot)>,
    revision: u64,
    root: Option<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn toggle(owner: &mut HostOwner) {
    if owner.phase != crate::bevy_shell::owner::Phase::Playing {
        return;
    }
    if let Some(play) = &owner.play {
        play.link.house_furniture_dispatch_bound(
            &play.link.house_furniture_snapshot(),
            Action::Toggle,
            owner.modal_active() || play.view.gameplay_input_blocked(&play.link),
        );
    }
}
fn inside(mut entity: Option<Entity>, root: Entity, parents: &Query<&ChildOf>) -> bool {
    while let Some(current) = entity {
        if current == root {
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
    let snapshot = owner
        .play
        .as_ref()
        .map(|play| play.link.house_furniture_snapshot())
        .filter(|snapshot| snapshot.panel.open);
    if state
        .snapshot
        .as_ref()
        .map(|(key, snapshot)| (key.generation, snapshot))
        == snapshot
            .as_ref()
            .map(|snapshot| (owner.generation(), snapshot))
    {
        return;
    }
    if let Some(root) = state.root.take() {
        // All focusable furniture controls are rebuilt with their bound snapshot.
        if inside(focus.get(), root, &parents) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    state.snapshot = None;
    let Some(snapshot) = snapshot else {
        return;
    };
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    let key = Key {
        generation: owner.generation(),
        revision,
    };
    state.root = Some(widgets::build(&mut commands, &skins, key, &snapshot.panel));
    state.snapshot = Some((key, snapshot));
}
fn perform(owner: &HostOwner, state: &State, control: Control) {
    let Some((key, snapshot)) = &state.snapshot else {
        return;
    };
    if *key != control.key
        || key.generation != owner.generation()
        || owner.phase != crate::bevy_shell::owner::Phase::Playing
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let action = match control.kind {
        Kind::Close => Action::Close,
        Kind::Select(index) => Action::Select(index),
        Kind::Request(intent) => Action::Request(intent),
    };
    play.link.house_furniture_dispatch_bound(
        snapshot,
        action,
        owner.modal_active() || play.view.gameplay_input_blocked(&play.link),
    );
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
        perform(&owner, &state, *control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    for event in events
        .read()
        .filter(|event| event.state == bevy::input::ButtonState::Pressed && !event.repeat)
    {
        if event.key_code == KeyCode::Escape
            && state.root.is_some_and(|root| guard.allows(root))
            && let Some((key, _)) = state.snapshot.as_ref()
        {
            perform(
                &owner,
                &state,
                Control {
                    key: *key,
                    kind: Kind::Close,
                },
            );
        }
    }
}
