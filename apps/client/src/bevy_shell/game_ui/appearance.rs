//! Outfit and mount editor over the retained appearance owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0025_appearance::view::{AppearanceControl as Kind, AppearanceSnapshot};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::EditableText,
    ui_widgets::Activate,
};
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
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Revision;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, AppearanceSnapshot)>,
    revision: u64,
    widgets: Option<widgets::Widgets>,
    pending_apply: Option<(Key, Entity)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link.appearance_open();
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
#[allow(clippy::too_many_arguments)] // Projection also owns focus cleanup and preview image handles.
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
    mut images: ResMut<Assets<Image>>,
) {
    let snapshot = owner
        .play
        .as_ref()
        .map(|play| play.link.appearance_snapshot());
    let snapshot = snapshot.filter(|snapshot| snapshot.panel.open && snapshot.panel.selected);
    if state.snapshot.as_ref().is_some_and(|(key, old)| {
        key.generation == owner.generation() && Some(old) == snapshot.as_ref()
    }) {
        return;
    }
    if let Some(widgets) = state.widgets.take() {
        if inside(focus.get(), widgets.overlay, &parents) {
            focus.clear();
        }
        commands.entity(widgets.overlay).despawn();
    }
    state.snapshot = None;
    state.pending_apply = None;
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
    let widgets = widgets::build(&mut commands, &skins, key, &snapshot, &mut images);
    focus.set(widgets.panel, FocusCause::Navigated);
    state.widgets = Some(widgets);
    state.snapshot = Some((key, snapshot));
}
fn perform(owner: &HostOwner, state: &State, key: Key, kind: Kind) {
    let Some((current, expected)) = &state.snapshot else {
        return;
    };
    if key != *current || key.generation != owner.generation() || owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if kind != Kind::Close && play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let _ = play.link.appearance_dispatch_bound(expected, kind);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        if matches!(control.kind, Kind::Apply(_)) {
            state.pending_apply = Some((control.key, event.entity));
        } else {
            perform(&owner, &state, control.key, control.kind.clone());
        }
    }
}
fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
    fields: Query<&EditableText, With<Revision>>,
) {
    let Some((key, entity)) = state.pending_apply.take() else {
        return;
    };
    if !guard.allows(entity) {
        return;
    }
    let Ok(field) = fields.single() else {
        return;
    };
    if field.pending_paste.is_some()
        || field
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
    {
        return;
    }
    perform(&owner, &state, key, Kind::Apply(field.value().to_string()));
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    let Some(widgets) = &state.widgets else {
        events.clear();
        return;
    };
    if !guard.allows(widgets.panel) {
        events.clear();
        return;
    }
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) && let Some((key, _)) = &state.snapshot
    {
        perform(&owner, &state, *key, Kind::Close);
    }
}
