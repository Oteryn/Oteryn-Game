//! Houses: retained listings, revision-bound access forms and server-owned transfers.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0049_house::view::{
    HouseBrowsingAction as Action, HouseBrowsingSnapshot as Snapshot,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::house::sale::Confirmation;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Request(usize),
    Map,
    Ownership,
    Page(i8),
    Select(usize),
    Review(u8),
    Confirm,
    CancelReview,
    Back,
    Close,
}
#[derive(Component)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Component)]
struct Field(usize);
#[derive(Resource, Default)]
struct State {
    snapshot: Option<Snapshot>,
    generation: u64,
    revision: u64,
    root: Option<Entity>,
    review: Option<Confirmation>,
    map: bool,
    feedback: String,
    dirty: bool,
    draft: Option<[String; 7]>,
    ownership_draft: Option<[String; 3]>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn available(owner: &HostOwner) -> bool {
    owner.phase == crate::bevy_shell::owner::Phase::Playing
        && !owner.modal_active()
        && owner
            .play
            .as_ref()
            .is_some_and(|p| !p.view.gameplay_input_blocked(&p.link))
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if available(owner)
        && let Some(play) = &owner.play
    {
        play.link.house_open();
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
    let snapshot = owner
        .play
        .as_ref()
        .map(|p| p.link.house_browsing_snapshot());
    let Some(snapshot) = snapshot.filter(|p| p.model.open) else {
        if let Some(root) = state.root.take() {
            if inside(focus.get(), root, &parents) {
                focus.clear();
            }
            commands.entity(root).despawn();
        }
        state.snapshot = None;
        state.review = None;
        state.map = false;
        state.draft = None;
        state.ownership_draft = None;
        state.feedback.clear();
        return;
    };
    if !state.dirty
        && state.generation == owner.generation()
        && state.snapshot.as_ref() == Some(&snapshot)
    {
        return;
    }
    if state.generation != owner.generation()
        || state.snapshot.as_ref().is_some_and(|old| old != &snapshot)
    {
        state.review = None;
        state.feedback.clear();
        state.draft = None;
        state.ownership_draft = None;
        if state.generation != owner.generation() {
            state.map = false;
        }
    }
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    if let Some(root) = state.root.take() {
        if inside(focus.get(), root, &parents) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    state.generation = owner.generation();
    state.revision = revision;
    state.snapshot = Some(snapshot);
    state.root = Some(widgets::build(&mut commands, &skins, &state));
    state.dirty = false;
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation != owner.generation()
        || control.revision != state.revision
        || !available(&owner)
    {
        return;
    }
    let Some(expected) = state.snapshot.as_ref() else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    // Never consume an edit/paste that Bevy has not yet committed to its text buffer.
    if fields.iter().any(|(_, text)| {
        text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
    }) {
        return;
    }
    let mut values = expected.fields.clone();
    let mut ownership = expected.ownership_fields.clone();
    for (field, text) in &fields {
        if let Some(slot) = values.get_mut(field.0) {
            *slot = text.value().to_string();
        } else if let Some(slot) = ownership.get_mut(field.0.saturating_sub(7)) {
            *slot = text.value().to_string();
        }
    }
    let saved_values = values.clone();
    let saved_ownership = ownership.clone();
    let action = match control.kind {
        Kind::Request(selection) => Action::Request {
            selection,
            fields: values,
        },
        Kind::Map => Action::Map { fields: values },
        Kind::Ownership => Action::OwnershipOpen,
        Kind::Page(direction) => Action::OwnershipPage(direction),
        Kind::Select(index) => Action::OwnershipSelect(index),
        Kind::Review(action) => Action::OwnershipReview {
            action,
            fields: ownership,
        },
        Kind::Confirm => {
            let Some(review) = state.review.clone() else {
                return;
            };
            Action::OwnershipConfirm {
                review,
                accepted: true,
            }
        }
        Kind::CancelReview => {
            let Some(review) = state.review.clone() else {
                return;
            };
            Action::OwnershipConfirm {
                review,
                accepted: false,
            }
        }
        Kind::Back => Action::OwnershipClose,
        Kind::Close => Action::Close,
    };
    match play.link.house_browsing_dispatch_bound(expected, action) {
        Ok(review) => {
            state.review = review;
            if control.kind == Kind::Map {
                state.map = true;
            }
            // A review is already bound by the owner; retain exactly its post-review snapshot.
            state.snapshot = Some(play.link.house_browsing_snapshot());
            state.feedback.clear();
            state.draft = None;
            state.ownership_draft = None;
        }
        Err(error) => {
            state.feedback = error;
            state.draft = Some(saved_values);
            state.ownership_draft = Some(saved_ownership);
        }
    }
    state.dirty = true;
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    for event in events.read() {
        if event.repeat || event.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        if event.key_code == KeyCode::Escape
            && let Some(root) = state.root
            && guard.allows(root)
            && available(&owner)
            && let Some(expected) = state.snapshot.as_ref()
            && let Some(play) = &owner.play
            && play
                .link
                .house_browsing_dispatch_bound(expected, Action::Close)
                .is_ok()
        {
            state.review = None;
        }
    }
}
