//! Expert fight modes presented over the existing PvP owner and dispatcher.
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    modal::ModalGuard,
};
use crate::play::feature_0087_pvp_expert::view::{
    ExpertPvpModesAction as Action, ExpertPvpModesSnapshot as Snapshot,
};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::pvp_expert::Panel;
#[cfg(test)]
mod tests;
mod widgets;
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    serial: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Field(usize, u8),
    Apply,
    Confirm,
    Cancel,
    Close,
}
#[derive(Component)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, Snapshot)>,
    serial: u64,
    root: Option<Entity>,
    confirming: bool,
    dirty: bool,
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
pub(in crate::bevy_shell) fn toggle(owner: &HostOwner) {
    if available(owner)
        && let Some(play) = &owner.play
    {
        let snapshot = play.link.expert_pvp_modes_snapshot();
        let action = if snapshot.panel.open {
            Action::Close
        } else {
            Action::Open
        };
        play.link.expert_pvp_modes_dispatch_bound(&snapshot, action);
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    parents: Query<&ChildOf>,
) {
    let snapshot = owner
        .play
        .as_ref()
        .map(|play| play.link.expert_pvp_modes_snapshot())
        .filter(|snapshot| snapshot.panel.open);
    let same = state.snapshot.as_ref().is_some_and(|(key, old)| {
        key.generation == owner.generation() && Some(old) == snapshot.as_ref()
    });
    if same && !state.dirty {
        return;
    }
    if let Some(root) = state.root.take() {
        let mut next = focus.get();
        while let Some(entity) = next {
            if entity == root {
                focus.clear();
                break;
            }
            next = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(root).despawn();
    }
    if !same {
        state.confirming = false;
    }
    state.dirty = false;
    state.snapshot = None;
    let Some(snapshot) = snapshot else {
        return;
    };
    let Some(serial) = state.serial.checked_add(1) else {
        return;
    };
    state.serial = serial;
    let key = Key {
        generation: owner.generation(),
        serial,
    };
    state.root = Some(widgets::build(
        &mut commands,
        &skins,
        key,
        &snapshot.panel,
        state.confirming,
    ));
    state.snapshot = Some((key, snapshot));
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control) {
    let Some((key, snapshot)) = &state.snapshot else {
        return;
    };
    if control.key != *key || key.generation != owner.generation() || !available(owner) {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    // Re-prove even presentation-only confirmation against the actual owner.
    if play.link.expert_pvp_modes_snapshot() != *snapshot {
        return;
    }
    let action = match control.kind {
        Kind::Close => Action::Close,
        Kind::Field(field, value) if !state.confirming => Action::Field(field, value),
        Kind::Apply if !state.confirming && snapshot.panel.selected && !snapshot.panel.pending => {
            state.confirming = true;
            state.dirty = true;
            return;
        }
        Kind::Confirm if state.confirming => Action::Apply,
        Kind::Cancel => {
            state.confirming = false;
            state.dirty = true;
            return;
        }
        _ => return,
    };
    play.link.expert_pvp_modes_dispatch_bound(snapshot, action);
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
    for event in events
        .read()
        .filter(|event| event.state == bevy::input::ButtonState::Pressed && !event.repeat)
    {
        if event.key_code == KeyCode::Escape
            && state.root.is_some_and(|root| guard.allows(root))
            && let Some((key, _)) = &state.snapshot
        {
            let control = Control {
                key: *key,
                kind: if state.confirming {
                    Kind::Cancel
                } else {
                    Kind::Close
                },
            };
            perform(&owner, &mut state, &control);
        }
    }
}
