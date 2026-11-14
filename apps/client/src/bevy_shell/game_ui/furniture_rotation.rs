//! Furniture rotation projects the retained catalogue and dispatches through its session owner.
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
use crate::play::feature_0445_house_furniture_rotation::view::{BoundAction, RotationBinding};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::house_furniture_rotation::{Intent, Panel};
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Select(usize),
    Request(Intent),
    Review,
    Confirm,
    Cancel,
    Close,
}
#[derive(Component, Clone)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, RotationBinding)>,
    revision: u64,
    confirmation: bool,
    dirty: bool,
    widgets: Option<widgets::Widgets>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn toggle(owner: &mut HostOwner) {
    if owner.phase == Phase::Playing
        && !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link.house_rotation_dispatch_bound(
            &play.link.house_rotation_binding(),
            BoundAction::Toggle,
        );
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
    let binding = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| play.link.house_rotation_binding())
        .filter(|binding| binding.panel.open);
    let Some(binding) = binding else {
        if let Some(widgets) = state.widgets.take() {
            clear_focus(&mut focus, widgets.overlay, &parents);
            commands.entity(widgets.overlay).despawn();
        }
        state.snapshot = None;
        state.confirmation = false;
        return;
    };
    let unchanged = state.snapshot.as_ref().is_some_and(|(key, old)| {
        key.generation == owner.generation()
            && old.same_session(&binding)
            && old.panel == binding.panel
    });
    if unchanged && !state.dirty {
        return;
    }
    if !unchanged {
        state.confirmation = false;
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
        clear_focus(&mut focus, widgets.overlay, &parents);
        commands.entity(widgets.overlay).despawn();
    }
    state.widgets = Some(widgets::build(
        &mut commands,
        &skins,
        key,
        &binding.panel,
        state.confirmation,
    ));
    state.snapshot = Some((key, binding));
    state.dirty = false;
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control) {
    let Some((key, binding)) = &state.snapshot else {
        return;
    };
    if *key != control.key
        || key.generation != owner.generation()
        || owner.phase != Phase::Playing
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
    let current = play.link.house_rotation_binding();
    if !current.same_session(binding) || current.panel != binding.panel {
        return;
    }
    let action = match control.kind {
        Kind::Review => {
            if binding.panel.selected && !binding.panel.pending && binding.can_rotate() {
                state.confirmation = true;
                state.dirty = true;
            }
            return;
        }
        Kind::Cancel => {
            state.confirmation = false;
            state.dirty = true;
            return;
        }
        Kind::Confirm if state.confirmation => BoundAction::Request(Intent::Rotate),
        Kind::Confirm => return,
        Kind::Select(index) => BoundAction::Select(index),
        Kind::Request(intent) => BoundAction::Request(intent),
        Kind::Close => BoundAction::Close,
    };
    play.link.house_rotation_dispatch_bound(binding, action);
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
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Some(widgets) = &state.widgets else {
        events.clear();
        return;
    };
    if !guard.allows(widgets.overlay) {
        events.clear();
        return;
    }
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) && let Some((key, _)) = &state.snapshot
    {
        let control = Control {
            key: *key,
            kind: if state.confirmation {
                Kind::Cancel
            } else {
                Kind::Close
            },
        };
        perform(&owner, &mut state, &control);
    }
}

fn clear_focus(focus: &mut InputFocus, overlay: Entity, parents: &Query<&ChildOf>) {
    let mut current = focus.get();
    while let Some(entity) = current {
        if entity == overlay {
            focus.clear();
            return;
        }
        current = parents.get(entity).ok().map(ChildOf::parent);
    }
}
