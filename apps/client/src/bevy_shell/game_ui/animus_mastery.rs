//! Paged mastery presentation over the retained Animus owner and native panel text.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0168_animus_mastery::view::{AnimusBinding, AnimusControl as Kind};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    ui_widgets::Activate,
};
use oteryn_client_ui::animus_mastery::{AnimusPanel as Panel, Intent};
#[derive(Component)]
struct Overlay;
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Component, Clone, Copy)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, AnimusBinding)>,
    revision: u64,
    widgets: Option<widgets::Widgets>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn available(owner: &HostOwner) -> bool {
    owner.phase == Phase::Playing
        && !owner.modal_active()
        && owner
            .play
            .as_ref()
            .is_some_and(|play| !play.view.gameplay_input_blocked(&play.link))
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if available(owner)
        && let Some(play) = &owner.play
    {
        let binding = play.link.animus_binding();
        play.link
            .animus_dispatch_bound(&binding, Kind::Toggle, true);
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
    let binding = owner.play.as_ref().map(|play| play.link.animus_binding());
    let Some(binding) = binding.filter(|binding| binding.panel.open) else {
        if let Some(widgets) = state.widgets.take() {
            if inside(focus.get(), widgets.overlay, &parents) {
                focus.clear();
            }
            commands.entity(widgets.overlay).despawn();
        }
        state.snapshot = None;
        return;
    };
    if state.snapshot.as_ref().is_some_and(|(key, old)| {
        key.generation == owner.generation()
            && !old.consumed()
            && old.same_session(&binding)
            && old.panel == binding.panel
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
    if let Some(widgets) = &state.widgets
        && inside(focus.get(), widgets.panel, &parents)
    {
        focus.set(widgets.panel, FocusCause::Navigated);
    }
    let widgets = state
        .widgets
        .get_or_insert_with(|| widgets::build(&mut commands, &skins));
    widgets::render(&mut commands, &skins, widgets, key, &binding.panel);
    state.snapshot = Some((key, binding));
}
fn perform(owner: &HostOwner, state: &State, control: Control, admitted: bool) {
    let Some((key, binding)) = &state.snapshot else {
        return;
    };
    if control.key != *key || key.generation != owner.generation() {
        return;
    }
    if let Some(play) = &owner.play {
        play.link
            .animus_dispatch_bound(binding, control.kind, admitted && available(owner));
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if let Ok(control) = controls.get(event.entity) {
        perform(&owner, &state, *control, guard.allows(event.entity));
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
        .filter(|event| !event.repeat && event.state == bevy::input::ButtonState::Pressed)
    {
        if event.key_code == KeyCode::Escape
            && let Some(widgets) = &state.widgets
            && let Some((key, _)) = &state.snapshot
        {
            perform(
                &owner,
                &state,
                Control {
                    key: *key,
                    kind: Kind::Close,
                },
                guard.allows(widgets.panel),
            );
        }
    }
}
