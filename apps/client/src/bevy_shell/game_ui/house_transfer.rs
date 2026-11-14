//! Presentation and input over the existing house-scope transfer owner.
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0086_house_transfer::view::HouseTransferSnapshot;
use bevy::{
    input::keyboard::KeyboardInput, input_focus::InputFocus, prelude::*, text::EditableText,
    ui_widgets::Activate,
};
use oteryn_client_ui::house_transfer::{Intent, Panel};
mod widgets;
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Request(Intent),
    Close,
}
#[derive(Component, Clone, Copy)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct HouseKey;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, HouseTransferSnapshot)>,
    revision: u64,
    root: Option<Entity>,
    pending: Option<(Control, Entity, Entity)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
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
        let expected = play.link.house_transfer_snapshot();
        play.link.house_transfer_set_open_bound(&expected, true);
    }
}
fn valid(owner: &HostOwner, state: &State, control: Control) -> bool {
    available(owner)
        && state.snapshot.as_ref().is_some_and(|(key, expected)| {
            *key == control.key
                && key.generation == owner.generation()
                && owner
                    .play
                    .as_ref()
                    .is_some_and(|play| play.link.house_transfer_snapshot() == *expected)
        })
}
fn perform(owner: &HostOwner, state: &State, control: Control, value: &str) {
    if !valid(owner, state, control) {
        return;
    }
    if let (Some(play), Some((_, expected))) = (&owner.play, &state.snapshot) {
        let intent = match control.kind {
            Kind::Request(intent) => Some(intent),
            Kind::Close => None,
        };
        play.link
            .house_transfer_dispatch_bound(expected, intent, value);
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<Entity, With<HouseKey>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) || !valid(&owner, &state, *control) {
        return;
    }
    if control.kind == Kind::Close {
        perform(&owner, &state, *control, "");
    } else if let Ok(field) = fields.single()
        && guard.allows(field)
        && state.pending.is_none()
    {
        // Read after EditableTextSystems, so same-frame typing/paste precedes submission.
        state.pending = Some((*control, event.entity, field));
    }
}
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<&EditableText, With<HouseKey>>,
    controls: Query<&Control>,
    guard: ModalGuard,
) {
    let Some((control, source, field)) = state.pending.take() else {
        return;
    };
    if !valid(&owner, &state, control)
        || !guard.allows(source)
        || !guard.allows(field)
        || !controls
            .get(source)
            .is_ok_and(|current| current.key == control.key && current.kind == control.kind)
    {
        return;
    }
    let Ok(text) = fields.get(field) else {
        return;
    };
    if editing(text) {
        state.pending = Some((control, source, field));
        return;
    }
    perform(&owner, &state, control, &text.value().to_string());
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
        .map(|play| play.link.house_transfer_snapshot())
        .filter(|snapshot| snapshot.panel.open && owner.phase == Phase::Playing);
    if state
        .snapshot
        .as_ref()
        .map(|(key, old)| (key.generation, old))
        == snapshot
            .as_ref()
            .map(|snapshot| (owner.generation(), snapshot))
    {
        return;
    }
    if let Some(root) = state.root.take() {
        if inside(focus.get(), root, &parents) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    state.pending = None;
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
fn keyboard(
    mut events: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    for event in events
        .read()
        .filter(|event| event.state == bevy::input::ButtonState::Pressed && !event.repeat)
    {
        let shortcut = event.key_code == KeyCode::F8
            && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
            && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
            && ![
                KeyCode::ShiftLeft,
                KeyCode::ShiftRight,
                KeyCode::SuperLeft,
                KeyCode::SuperRight,
            ]
            .iter()
            .any(|key| keys.pressed(*key));
        if !shortcut && event.key_code != KeyCode::Escape {
            continue;
        }
        if let (Some(root), Some((key, _))) = (state.root, &state.snapshot) {
            if guard.allows(root) {
                perform(
                    &owner,
                    &state,
                    Control {
                        key: *key,
                        kind: Kind::Close,
                    },
                    "",
                );
            }
        } else if shortcut && guard.allows_focus(None) {
            open(&owner);
        }
    }
}

#[cfg(test)]
mod tests;
