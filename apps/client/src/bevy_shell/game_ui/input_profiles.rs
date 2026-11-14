//! Local profile/context/keybinding and shortcut editors over the retained settings owner.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::{PreferenceBinding, ReferencePreferences, SettingsPage};
use bevy::{
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui_bevy as source;
#[cfg(test)]
mod tests;
mod widgets;
#[derive(Clone, Debug, PartialEq, Eq)]
enum Kind {
    Profile(String),
    Context(bool),
    Action(String),
    Copy,
    Rename,
    Delete,
    ConfirmDelete,
    CancelDelete,
    Primary,
    Secondary,
    ShortcutAdd(String),
    ShortcutRemove(usize),
    ShortcutMove(usize, usize),
    Close,
}
#[derive(Clone, Copy)]
struct Key {
    generation: u64,
    scope: Option<[u8; 16]>,
    revision: u64,
}
#[derive(Component, Clone)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct Field(usize);
#[derive(Clone)]
struct Snapshot {
    generation: u64,
    scope: Option<[u8; 16]>,
    preferences: ReferencePreferences,
}
#[derive(Resource, Default)]
struct State {
    root: Option<Entity>,
    snapshot: Option<Snapshot>,
    revision: u64,
    selected: Option<String>,
    confirm: Option<[String; 3]>,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(
            PostUpdate,
            invalidate_confirmation.after(bevy::text::EditableTextSystems),
        );
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if owner.play.is_some() && !owner.modal_active() {
        owner.settings.page = SettingsPage::Hotkeys;
        owner.settings.open = true;
    }
}
fn is_open(owner: &HostOwner) -> bool {
    owner.play.is_some()
        && owner.settings.open
        && owner.settings.page == SettingsPage::Hotkeys
        && owner.preferences_draft().is_none()
}
fn bindings() -> Vec<PreferenceBinding> {
    crate::play::preference_bindings()
        .unwrap_or_default()
        .into_iter()
        .filter(|binding| !binding.default_chord.starts_with("MOUSE"))
        .collect()
}
fn binding_label(binding: &PreferenceBinding) -> String {
    binding
        .id
        .split_once('-')
        .map_or(binding.id, |(_, name)| name)
        .trim_end_matches("-open")
        .replace('-', " ")
}
fn project(
    mut commands: Commands,
    mut owner: NonSendMut<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    if state.snapshot.as_ref().is_some_and(|old| {
        old.generation != owner.generation() || old.scope != owner.settings.character_scope()
    }) {
        owner.settings.open = false;
    }
    if !is_open(&owner) {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.snapshot = None;
        state.confirm = None;
        return;
    }
    let preferences = &owner.settings.preferences;
    let changed = state
        .snapshot
        .as_ref()
        .is_none_or(|old| old.preferences != *preferences);
    if !changed && !state.dirty {
        return;
    }
    if changed {
        state.confirm = None;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.revision = state.revision.saturating_add(1);
    state.dirty = false;
    state.snapshot = Some(Snapshot {
        generation: owner.generation(),
        scope: owner.settings.character_scope(),
        preferences: preferences.clone(),
    });
    if state.selected.is_none() {
        state.selected = bindings().first().map(|binding| binding.id.into());
    }
    widgets::build(&mut commands, &skins, &owner, &mut state);
}
// NonSendMut<HostOwner> is the main-thread owner lock. All admission and the one
// local mutation occur inside this exclusive borrow; no mailbox or second store exists.
fn perform(
    owner: &mut HostOwner,
    state: &mut State,
    control: &Control,
    fields: &[String; 3],
    admitted: bool,
) {
    let Some(snapshot) = &state.snapshot else {
        return;
    };
    if control.key.generation != owner.generation()
        || control.key.scope != owner.settings.character_scope()
        || snapshot.generation != owner.generation()
        || snapshot.scope != owner.settings.character_scope()
        || !is_open(owner)
    {
        return;
    }
    // Close depends only on the live session, including while blocked or unavailable.
    if control.kind == Kind::Close {
        owner.settings.open = false;
        state.confirm = None;
        return;
    }
    if !admitted
        || state.dirty
        || control.key.revision != state.revision
        || snapshot.preferences != owner.settings.preferences
        || owner.modal_active()
        || owner
            .play
            .as_ref()
            .is_none_or(|play| play.view.gameplay_input_blocked(&play.link))
    {
        return;
    }
    if state.confirm.as_ref().is_some_and(|draft| draft != fields) {
        state.confirm = None;
    }
    if let Kind::Action(id) = &control.kind {
        state.selected = Some(id.clone());
        state.confirm = None;
        state.dirty = true;
        return;
    }
    if control.kind == Kind::Delete {
        state.confirm = Some(fields.clone());
        state.dirty = true;
        return;
    }
    if control.kind == Kind::CancelDelete
        || (control.kind == Kind::ConfirmDelete && state.confirm.is_none())
    {
        state.confirm = None;
        state.dirty = true;
        return;
    }
    let mut candidate = owner.settings.preferences.clone();
    let selected = candidate.hotkey_contexts.selected.clone();
    let result = match &control.kind {
        Kind::Profile(name) => candidate.select_hotkey_profile(name),
        Kind::Context(chat) => candidate.select_hotkey_context(*chat),
        Kind::Copy => candidate.add_hotkey_profile(fields[0].trim()),
        Kind::Rename => candidate.rename_hotkey_profile(&selected, fields[0].trim()),
        Kind::ConfirmDelete => candidate.remove_hotkey_profile(&selected),
        Kind::Primary => candidate.rebind(state.selected.as_deref().unwrap_or(""), &fields[1]),
        Kind::Secondary => {
            candidate.rebind_secondary(state.selected.as_deref().unwrap_or(""), &fields[2])
        }
        Kind::ShortcutAdd(id) => candidate.add_shortcut(id),
        Kind::ShortcutRemove(row) => candidate.remove_shortcut(*row),
        Kind::ShortcutMove(row, to) => candidate.move_shortcut(*row, *to),
        _ => return,
    };
    match result {
        Ok(()) => owner.settings.apply(candidate),
        Err(error) => owner.settings.feedback = error.to_string(),
    }
    state.confirm = None;
    state.dirty = true;
}
fn values(fields: &Query<(&Field, &EditableText)>) -> [String; 3] {
    let mut values = std::array::from_fn(|_| String::new());
    for (field, value) in fields {
        values[field.0] = value.editor.text().to_string();
    }
    values
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
) {
    if let Ok(control) = controls.get(event.entity) {
        perform(
            &mut owner,
            &mut state,
            control,
            &values(&fields),
            guard.allows(event.entity),
        );
    }
}
fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
) {
    if keys.read().any(|key| {
        key.key_code == KeyCode::Escape
            && key.state == bevy::input::ButtonState::Pressed
            && !key.repeat
    }) {
        let control = Control {
            key: Key {
                generation: owner.generation(),
                scope: owner.settings.character_scope(),
                revision: state.revision,
            },
            kind: Kind::Close,
        };
        perform(
            &mut owner,
            &mut state,
            &control,
            &std::array::from_fn(|_| String::new()),
            true,
        );
    }
}

fn invalidate_confirmation(fields: Query<(&Field, &EditableText)>, mut state: ResMut<State>) {
    if state
        .confirm
        .as_ref()
        .is_some_and(|draft| *draft != values(&fields))
    {
        state.confirm = None;
        state.dirty = true;
    }
}
