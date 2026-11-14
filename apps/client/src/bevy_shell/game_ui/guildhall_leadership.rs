//! Guildhall leadership presentation over the retained property owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::guildhall::Panel;
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Field(usize);
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Transfer,
    Confirm,
    Cancel,
    Close,
}
#[derive(Component, Clone, Copy)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, Panel)>,
    revision: u64,
    root: Option<Entity>,
    confirmation: Option<[String; 3]>,
    draft: Option<[String; 3]>,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn toggle(owner: &mut HostOwner) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play {
        let panel = play.link.guildhall_panel();
        if panel.open {
            let _ = play.link.guildhall_leadership_close_bound(&panel);
        } else {
            play.link.guildhall_open();
        }
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
        .map(|p| (owner.generation(), p.link.guildhall_panel()))
        .filter(|(_, p)| p.open);
    if state.snapshot == snapshot && !state.dirty {
        return;
    }
    if state.snapshot != snapshot {
        state.confirmation = None;
        state.draft = None;
    }
    if let Some(root) = state.root.take() {
        if inside(focus.get(), root, &parents) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    state.snapshot = snapshot;
    state.dirty = false;
    let Some((generation, mut panel)) = state.snapshot.clone() else {
        return;
    };
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    if let Some(fields) = &state.draft {
        panel.fields.clone_from(fields);
    }
    state.root = Some(widgets::build(
        &mut commands,
        &skins,
        &panel,
        generation,
        revision,
        state.confirmation.as_ref(),
    ));
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
    perform(&owner, &mut state, control, &fields);
}
fn perform(
    owner: &HostOwner,
    state: &mut State,
    control: &Control,
    fields: &Query<(&Field, &EditableText)>,
) {
    let Some((generation, expected)) = &state.snapshot else {
        return;
    };
    if *generation != owner.generation()
        || control.generation != *generation
        || control.revision != state.revision
        || owner.modal_active()
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.link.guildhall_panel() != *expected || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match control.kind {
        Kind::Close => {
            let _ = play.link.guildhall_leadership_close_bound(expected);
        }
        Kind::Transfer => {
            if !expected.selected || expected.pending || state.confirmation.is_some() {
                return;
            }
            let mut values = expected.fields.clone();
            for (field, text) in fields.iter() {
                if text.pending_paste.is_some()
                    || text
                        .pending_edits
                        .iter()
                        .any(bevy::text::TextEdit::is_destructive)
                {
                    return;
                }
                values[field.0] = text.value().to_string();
            }
            state.confirmation = Some(values);
            state.dirty = true;
        }
        Kind::Confirm => {
            if let Some(values) = state.confirmation.as_ref() {
                let _ =
                    play.link
                        .guildhall_leadership_transfer_bound(expected, values.clone(), true);
            }
            state.confirmation = None;
            state.dirty = true;
        }
        Kind::Cancel => {
            state.draft = state.confirmation.take();
            state.dirty = true;
        }
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
) {
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        if event.key_code == KeyCode::KeyG
            && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
            && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
            && !keys.pressed(KeyCode::ShiftLeft)
            && !keys.pressed(KeyCode::ShiftRight)
            && guard.allows_focus(state.root)
        {
            toggle(&mut owner);
        }
        if event.key_code == KeyCode::Escape
            && let Some(root) = state.root
            && guard.allows(root)
        {
            let kind = if state.confirmation.is_some() {
                Kind::Cancel
            } else {
                Kind::Close
            };
            let control = Control {
                generation: owner.generation(),
                revision: state.revision,
                kind,
            };
            perform(&owner, &mut state, &control, &fields);
        }
    }
}
