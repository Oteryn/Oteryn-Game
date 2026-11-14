//! Browse Field projects the retained feature owner; the server owns every item change.
#[cfg(test)]
mod tests;
mod widgets;
use super::{GameModal, modal::ModalGuard};
use crate::{
    bevy_shell::{owner::HostOwner, ui::Skins},
    play::feature_0230_browse_field::view::{
        BrowseFieldAction as Action, BrowseFieldBinding as Binding,
    },
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
#[cfg(test)]
use oteryn_client_ui::browse_field::BrowseFieldPanel as Panel;
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Close,
    Open,
    Seek,
    Item(usize),
    Confirm,
    Cancel,
}
#[derive(Component)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct Field(usize);
#[derive(Resource)]
struct State {
    snapshot: Option<(Key, Binding)>,
    fields: [String; 7],
    confirmation: Option<[String; 7]>,
    revision: u64,
    root: Option<Entity>,
    dirty: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            snapshot: None,
            fields: std::array::from_fn(|_| "0".into()),
            confirmation: None,
            revision: 0,
            root: None,
            dirty: false,
        }
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link.browse_field_set_open_bound(true);
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
    fields: Query<(&Field, &EditableText)>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    for (field, text) in &fields {
        state.fields[field.0] = text.editor.text().to_string();
    }
    let binding = owner.play.as_ref().map(|p| p.link.browse_field_binding());
    let Some(binding) = binding.filter(|b| b.panel.open) else {
        if let Some(root) = state.root.take() {
            if inside(focus.get(), root, &parents) {
                focus.clear();
            }
            commands.entity(root).despawn();
        }
        state.snapshot = None;
        state.confirmation = None;
        return;
    };
    let unchanged = state.snapshot.as_ref().is_some_and(|(key, old)| {
        key.generation == owner.generation()
            && old.panel == binding.panel
            && owner.play.as_ref().is_some_and(|p| old.is_for(&p.link))
    });
    if unchanged && !state.dirty {
        return;
    }
    if !unchanged {
        state.confirmation = None;
        if state.snapshot.as_ref().is_none_or(|(key, old)| {
            key.generation != owner.generation()
                || !owner.play.as_ref().is_some_and(|p| old.is_for(&p.link))
        }) {
            state.fields = [
                binding.panel.x.to_string(),
                binding.panel.y.to_string(),
                binding.panel.floor.to_string(),
                binding.panel.view_id.to_string(),
                binding.panel.index.to_string(),
                "0".into(),
                "0".into(),
            ];
        } else if state
            .snapshot
            .as_ref()
            .is_some_and(|(_, old)| old.panel.view_id != binding.panel.view_id)
        {
            state.fields[3] = binding.panel.view_id.to_string();
        }
    }
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    let key = Key {
        generation: owner.generation(),
        revision,
    };
    if let Some(root) = state.root.take() {
        if inside(focus.get(), root, &parents) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    state.root = Some(widgets::build(
        &mut commands,
        &skins,
        key,
        &binding,
        &state.fields,
        state.confirmation.is_some(),
    ));
    state.snapshot = Some((key, binding));
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
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) {
        return;
    }
    for (field, text) in &fields {
        state.fields[field.0] = text.editor.text().to_string();
    }
    perform(&owner, &mut state, control);
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control) {
    let Some((key, binding)) = &state.snapshot else {
        return;
    };
    if control.key != *key || key.generation != owner.generation() || owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if !binding.is_for(&play.link)
        || play.view.gameplay_input_blocked(&play.link)
        || play.link.browse_field_binding().panel != binding.panel
    {
        return;
    }
    let action = match control.kind {
        Kind::Item(106) => {
            if binding.panel.selected && !binding.panel.pending {
                state.confirmation = Some(state.fields.clone());
                state.dirty = true;
            }
            return;
        }
        Kind::Cancel => {
            state.confirmation = None;
            state.dirty = true;
            return;
        }
        Kind::Confirm => {
            if state.confirmation.as_ref() != Some(&state.fields) {
                return;
            }
            Action::Item(106)
        }
        Kind::Item(id) => Action::Item(id),
        Kind::Open => Action::Open,
        Kind::Seek => Action::Seek,
        Kind::Close => Action::Close,
    };
    if play.link.browse_field_dispatch_bound(
        binding,
        action,
        &state.fields,
        control.kind == Kind::Confirm,
    ) {
        state.confirmation = None;
        state.dirty = true;
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        if event.key_code == KeyCode::KeyB
            && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
            && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
            && !keys.pressed(KeyCode::AltLeft)
            && !keys.pressed(KeyCode::AltRight)
        {
            if let Some(root) = state.root {
                if guard.allows(root)
                    && let Some((key, _)) = &state.snapshot
                {
                    let control = Control {
                        key: *key,
                        kind: Kind::Close,
                    };
                    perform(&owner, &mut state, &control);
                }
            } else if guard.allows_focus(None) {
                open(&owner);
            }
        }
        if event.key_code == KeyCode::Escape
            && let Some(root) = state.root
            && guard.allows(root)
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
            perform(&owner, &mut state, &control);
        }
    }
}
