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
    Confirm,
    Cancel,
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
    confirmation: Option<(Intent, String)>,
    draft: Option<String>,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
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
                && (control.kind == Kind::Close
                    || expected.panel.selected && !expected.panel.pending)
                && owner
                    .play
                    .as_ref()
                    .is_some_and(|play| play.link.house_transfer_snapshot() == *expected)
        })
}
fn perform(owner: &HostOwner, state: &mut State, control: Control) {
    if !valid(owner, state, control) {
        return;
    }
    let request = match control.kind {
        Kind::Confirm => state
            .confirmation
            .take()
            .map(|(intent, house)| (Some(intent), house)),
        Kind::Close => Some((None, String::new())),
        _ => return,
    };
    let Some((intent, house)) = request else {
        return;
    };
    // Consume consent before the atomic owner check, including a refusal.
    state.confirmation = None;
    state.pending = None;
    state.dirty = true;
    if let (Some(play), Some((_, expected))) = (&owner.play, &state.snapshot) {
        play.link
            .house_transfer_dispatch_bound(expected, intent, &house);
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
    if matches!(control.kind, Kind::Close | Kind::Confirm) {
        perform(&owner, &mut state, *control);
    } else if control.kind == Kind::Cancel {
        state.confirmation = None;
        state.dirty = true;
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
    if let Kind::Request(intent) = control.kind {
        let house = text.value().to_string();
        state.draft = Some(house.clone());
        state.confirmation = Some((intent, house));
        state.dirty = true;
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
        .map(|play| play.link.house_transfer_snapshot())
        .filter(|snapshot| snapshot.panel.open && owner.phase == Phase::Playing);
    let unchanged = state
        .snapshot
        .as_ref()
        .map(|(key, old)| (key.generation, old))
        == snapshot
            .as_ref()
            .map(|snapshot| (owner.generation(), snapshot));
    if unchanged && !state.dirty {
        return;
    }
    if !unchanged {
        state.confirmation = None;
        state.draft = None;
    }
    state.dirty = false;
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
    state.root = Some(widgets::build(
        &mut commands,
        &skins,
        key,
        &snapshot.panel,
        state.confirmation.as_ref(),
        state.draft.as_deref(),
    ));
    state.snapshot = Some((key, snapshot));
}
fn keyboard(
    mut events: MessageReader<KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) && let (Some(root), Some((key, _))) = (state.root, &state.snapshot)
        && guard.allows(root)
    {
        let control = Control {
            key: *key,
            kind: Kind::Close,
        };
        perform(&owner, &mut state, control);
    }
}

#[cfg(test)]
mod tests;
