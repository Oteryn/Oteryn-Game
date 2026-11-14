//! Cast settings and character consent over the existing session-owned presenter.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use crate::play::feature_0290_cast_control::view::CastControlSnapshot;
use bevy::{
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::Activate,
};
use oteryn_client_ui::cast_control::Panel;

#[derive(Component, Clone, Copy)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Request(usize),
    Confirm,
    Cancel,
    Close,
}
#[derive(Component)]
struct Field(usize);
#[derive(Resource, Default)]
struct State {
    snapshot: Option<CastControlSnapshot>,
    generation: u64,
    revision: u64,
    root: Option<Entity>,
    confirmation: Option<(usize, [String; 4])>,
    queued: Option<(Entity, Control)>,
    dirty: bool,
    feedback: String,
}
fn presentation(panel: &Panel) -> String {
    oteryn_client_ui::cast_consent::panel_text(panel)
}
fn settings(panel: &Panel) -> [String; 4] {
    [
        panel.delay.to_string(),
        panel.cap.to_string(),
        u8::from(panel.password).to_string(),
        panel.access_revision.to_string(),
    ]
}
fn needs_confirmation(action: usize) -> bool {
    matches!(action, 0 | 1 | 3 | 4)
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link.cast_control_toggle_bound();
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && state.queued.is_none()
        && let Ok(control) = controls.get(event.entity)
    {
        state.queued = Some((event.entity, *control));
    }
}
fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
    fields: Query<(&Field, &EditableText)>,
) {
    let Some((entity, control)) = state.queued.take() else {
        return;
    };
    if !guard.allows(entity)
        || owner.modal_active()
        || control.generation != owner.generation()
        || state.generation != owner.generation()
        || control.revision != state.revision
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let Some(snapshot) = state.snapshot.clone() else {
        return;
    };
    if !snapshot.same(&play.link.cast_control_snapshot()) || !snapshot.panel.open {
        return;
    }
    match control.kind {
        Kind::Close => {
            play.link.cast_control_close_bound(&snapshot);
        }
        Kind::Cancel => {
            state.confirmation = None;
            state.dirty = true;
        }
        Kind::Request(action) => {
            if snapshot.panel.pending {
                return;
            }
            let mut draft = settings(&snapshot.panel);
            for (field, text) in &fields {
                // Do not review a value while an IME or queued edit is changing it.
                if text.editor.is_composing()
                    || text.pending_paste.is_some()
                    || text
                        .pending_edits
                        .iter()
                        .any(bevy::text::TextEdit::is_destructive)
                {
                    return;
                }
                if let Some(value) = draft.get_mut(field.0) {
                    *value = text.value().to_string();
                }
            }
            if needs_confirmation(action) {
                state.confirmation = Some((action, draft));
                state.dirty = true;
            } else if play
                .link
                .cast_control_dispatch_bound(&snapshot, action, &draft, false)
            {
                state.revision = state.revision.wrapping_add(1);
                state.dirty = true;
            }
        }
        Kind::Confirm => {
            if let Some((action, draft)) = state.confirmation.take() {
                let sent = play
                    .link
                    .cast_control_dispatch_bound(&snapshot, action, &draft, true);
                state.feedback = if sent {
                    String::new()
                } else {
                    "Request rejected; check the settings and refresh cast or consent.".into()
                };
                state.revision = state.revision.wrapping_add(1);
                state.dirty = true;
            }
        }
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
    let current = owner
        .play
        .as_ref()
        .map(|play| play.link.cast_control_snapshot());
    let changed = state.generation != owner.generation()
        || match (&state.snapshot, &current) {
            (Some(old), Some(new)) => !old.same(new),
            (None, None) => false,
            _ => true,
        };
    if !changed && !state.dirty {
        return;
    }
    if changed {
        state.confirmation = None;
        state.queued = None;
        state.feedback.clear();
    }
    state.dirty = false;
    state.generation = owner.generation();
    state.revision = state.revision.wrapping_add(1);
    if let Some(root) = state.root.take() {
        let mut current = focus.get();
        while let Some(entity) = current {
            if entity == root {
                focus.clear();
                break;
            }
            current = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(root).despawn();
    }
    state.snapshot = current;
    if let Some(snapshot) = &state.snapshot
        && snapshot.panel.open
    {
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            &state,
            &snapshot.panel,
        ));
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<bevy::input_focus::InputFocus>,
    fields: Query<(), With<EditableText>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        if event.key_code == KeyCode::KeyS
            && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
            && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
            && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
            && !focus.get().is_some_and(|entity| fields.contains(entity))
            && !owner
                .play
                .as_ref()
                .is_some_and(|play| play.link.preference_chat_open())
            && guard.allows_focus(state.root)
        {
            open(&owner);
        }
        if event.key_code == KeyCode::Escape
            && let Some(root) = state.root
            && guard.allows(root)
        {
            state.queued = Some((
                root,
                Control {
                    generation: state.generation,
                    revision: state.revision,
                    kind: Kind::Close,
                },
            ));
        }
    }
}
