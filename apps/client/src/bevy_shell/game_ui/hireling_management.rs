//! Hireling management presentation over the canonical feature-250 and Store owners.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0250_hireling_management::view::{
    HirelingAction, HirelingReview, HirelingSnapshot,
};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::EditableText,
    ui_widgets::Activate,
};
use oteryn_client_ui::hireling_management as ui;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Close,
    Select(usize),
    Refresh,
    Return,
    Save(u16),
    Prices,
    Configure(usize),
    Confirm,
    Cancel,
}
#[derive(Component, Clone, Copy)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct Overlay;
#[derive(Component, Clone, Copy)]
enum Field {
    Name,
    Color(usize),
}
#[derive(Clone)]
enum Confirmation {
    Return(String),
    Hireling(Box<HirelingReview>),
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, HirelingSnapshot)>,
    revision: u64,
    widgets: Option<widgets::Widgets>,
    confirmation: Option<Confirmation>,
    dirty: bool,
    feedback: String,
    name: String,
    colors: [String; 4],
    pending: Option<(Control, Entity)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play {
        let snapshot = play.link.hireling_management_snapshot();
        if snapshot.panel.open {
            play.link
                .hireling_management_dispatch_bound(&snapshot, HirelingAction::Close);
        } else if snapshot.live && !play.view.gameplay_input_blocked(&play.link) {
            play.link
                .hireling_management_dispatch_bound(&snapshot, HirelingAction::Open);
        }
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
    fields: Query<(&Field, &EditableText)>,
) {
    let snapshot = owner
        .play
        .as_ref()
        .map(|p| p.link.hireling_management_snapshot());
    let Some(snapshot) = snapshot.filter(|s| s.live && s.panel.open) else {
        if let Some(widgets) = state.widgets.take() {
            if inside(focus.get(), widgets.overlay, &parents) {
                focus.clear();
            }
            commands.entity(widgets.overlay).despawn();
        }
        state.snapshot = None;
        state.name.clear();
        state.colors = Default::default();
        state.confirmation = None;
        state.pending = None;
        return;
    };
    let same = state
        .snapshot
        .as_ref()
        .is_some_and(|(key, old)| key.generation == owner.generation() && old == &snapshot);
    if same && !state.dirty {
        return;
    }
    let same_session = state.snapshot.as_ref().is_some_and(|(key, old)| {
        key.generation == owner.generation() && old.same_session(&snapshot)
    });
    // Keep the native editor drafts across owner replies, after pending edits have settled.
    if same_session && !read_fields(&mut state, &fields) {
        return;
    }
    if !same {
        if !same_session {
            state.name.clear();
        }
        let entries_changed = state.snapshot.as_ref().is_none_or(|(_, old)| {
            !same_session
                || old.panel.entries != snapshot.panel.entries
                || old.panel.identity != snapshot.panel.identity
        });
        if entries_changed
            && let Some(entry) = snapshot
                .panel
                .entries
                .iter()
                .find(|e| Some(e.hireling) == snapshot.panel.identity)
        {
            state.colors = entry.colors.map(|c| c.to_string());
        }
        state.confirmation = None;
        state.pending = None;
        state.feedback.clear();
    }
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    let key = Key {
        generation: owner.generation(),
        revision,
    };
    if state
        .snapshot
        .as_ref()
        .is_some_and(|(old, _)| old.generation != key.generation)
        && let Some(widgets) = state.widgets.take()
    {
        if inside(focus.get(), widgets.overlay, &parents) {
            focus.clear();
        }
        commands.entity(widgets.overlay).despawn();
    }
    let mut widgets = state
        .widgets
        .take()
        .unwrap_or_else(|| widgets::build(&mut commands, &skins));
    if widgets
        .content
        .is_some_and(|content| inside(focus.get(), content, &parents))
    {
        focus.set(widgets.panel, FocusCause::Navigated);
    }
    widgets::render(&mut commands, &skins, &mut widgets, key, &snapshot, &state);
    state.widgets = Some(widgets);
    state.snapshot = Some((key, snapshot));
    state.dirty = false;
}
fn valid(owner: &HostOwner, state: &State, control: &Control) -> bool {
    state
        .snapshot
        .as_ref()
        .is_some_and(|(key, _)| *key == control.key && key.generation == owner.generation())
        && !owner.modal_active()
        && owner
            .play
            .as_ref()
            .is_some_and(|p| !p.view.gameplay_input_blocked(&p.link))
}
fn perform(owner: &HostOwner, state: &mut State, control: Control) {
    if !valid(owner, state, &control) {
        return;
    }
    let Some((_, expected)) = &state.snapshot else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if play.link.hireling_management_snapshot() != *expected {
        return;
    }
    if state.confirmation.is_some()
        && !matches!(control.kind, Kind::Confirm | Kind::Cancel | Kind::Close)
    {
        return;
    }
    let action = match control.kind {
        Kind::Close => HirelingAction::Close,
        Kind::Select(index) => HirelingAction::Select(index),
        Kind::Refresh => HirelingAction::Refresh,
        Kind::Prices => HirelingAction::Prices,
        Kind::Save(look_type) => {
            let colors = state.colors.each_ref().map(|c| c.parse::<u8>().ok());
            let [Some(a), Some(b), Some(c), Some(d)] = colors else {
                state.feedback =
                    "Select a hireling and outfit; enter four colors from 0 to 255".into();
                state.dirty = true;
                return;
            };
            HirelingAction::Outfit {
                look_type,
                colors: [a, b, c, d],
            }
        }
        Kind::Return => {
            if expected.panel.pending.is_some()
                || expected.store.pending
                || expected.panel.identity.is_none()
            {
                return;
            }
            let Some(entry) = expected
                .panel
                .entries
                .iter()
                .find(|entry| Some(entry.hireling) == expected.panel.identity)
            else {
                return;
            };
            state.confirmation = Some(Confirmation::Return(entry.name.clone()));
            state.dirty = true;
            return;
        }
        Kind::Configure(action) => {
            match play
                .link
                .hireling_management_review_bound(expected, action, &state.name)
            {
                Ok(review) => state.confirmation = Some(Confirmation::Hireling(Box::new(review))),
                Err(message) => state.feedback = message.into(),
            }
            state.dirty = true;
            return;
        }
        Kind::Confirm => {
            let Some(confirmation) = state.confirmation.take() else {
                return;
            };
            state.dirty = true;
            match confirmation {
                Confirmation::Return(_) => HirelingAction::Return,
                Confirmation::Hireling(review) => HirelingAction::Configure(review),
            }
        }
        Kind::Cancel => {
            state.confirmation = None;
            state.dirty = true;
            return;
        }
    };
    play.link
        .hireling_management_dispatch_bound(expected, action);
}
fn needs_fields(kind: Kind) -> bool {
    !matches!(kind, Kind::Close | Kind::Confirm | Kind::Cancel)
}
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
fn read_fields(state: &mut State, fields: &Query<(&Field, &EditableText)>) -> bool {
    if fields.iter().any(|(_, text)| editing(text)) {
        return false;
    }
    for (field, text) in fields.iter() {
        match field {
            Field::Name => state.name = text.value().to_string(),
            Field::Color(index) => {
                if let Some(color) = state.colors.get_mut(*index) {
                    *color = text.value().to_string();
                }
            }
        }
    }
    true
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        if !valid(&owner, &state, control) {
            return;
        }
        if needs_fields(control.kind) && !read_fields(&mut state, &fields) {
            state.pending = Some((*control, event.entity));
            return;
        }
        state.pending = None;
        perform(&owner, &mut state, *control);
    }
}
fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
) {
    let Some((control, source)) = state.pending.take() else {
        return;
    };
    if !guard.allows(source) || !valid(&owner, &state, &control) {
        return;
    }
    if read_fields(&mut state, &fields) {
        perform(&owner, &mut state, control);
    } else {
        state.pending = Some((control, source));
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
) {
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        if event.key_code == KeyCode::F8
            && state.widgets.as_ref().map_or_else(
                || guard.allows_focus(None),
                |widgets| guard.allows(widgets.panel),
            )
        {
            open(&mut owner);
        } else if event.key_code == KeyCode::Escape
            && let Some(widgets) = &state.widgets
            && guard.allows(widgets.panel)
            && let Some((key, _)) = &state.snapshot
        {
            let control = Control {
                key: *key,
                kind: if state.confirmation.is_some() {
                    Kind::Cancel
                } else {
                    Kind::Close
                },
            };
            perform(&owner, &mut state, control);
        }
    }
}
