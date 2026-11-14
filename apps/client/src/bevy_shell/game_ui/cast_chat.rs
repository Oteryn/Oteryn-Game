//! Cast chat presentation over the retained Win32 owner and dispatcher.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use crate::play::feature_0291_cast_chat::view::CastChatBound;
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
#[derive(Component)]
struct Overlay;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Reply,
    Subject,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Send(u8),
    Confirm,
    Cancel,
    Close,
}
#[derive(Component)]
struct Control {
    revision: u64,
    kind: Kind,
}
#[derive(Clone)]
struct Confirmation {
    text: String,
    subject: String,
    action: u8,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    revision: u64,
    bound: Option<CastChatBound>,
    root: Option<Entity>,
    confirmation: Option<Confirmation>,
    pending: Option<(u64, Kind)>,
    reply: String,
    subject: String,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, keyboard)
        .add_systems(
            PostUpdate,
            (finish, project)
                .chain()
                .after(bevy::text::EditableTextSystems),
        );
}
pub(in crate::bevy_shell) fn toggle(owner: &mut HostOwner) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play {
        let panel = play.link.cast_chat_panel();
        let _ = play.link.cast_chat_set_open(!panel.open);
    }
}
pub(in crate::bevy_shell) fn shortcut(
    owner: &mut HostOwner,
    events: &[oteryn_input_actions::NormalizedInputEvent],
) -> bool {
    if !oteryn_client_ui::cast_chat::open_key(events) {
        return false;
    }
    toggle(owner);
    true
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(&Field, &EditableText)>,
) {
    let bound = owner
        .play
        .as_ref()
        .map(|play| play.link.cast_chat_bound())
        .filter(|bound| bound.panel.open);
    let changed = state.generation != owner.generation()
        || match (&state.bound, &bound) {
            (Some(old), Some(new)) => !old.same_view(new),
            (None, None) => false,
            _ => true,
        };
    if !changed && !state.dirty {
        return;
    }
    if state.generation == owner.generation() && bound.is_some() {
        for (field_kind, field) in &fields {
            match field_kind {
                Field::Reply => state.reply = field.value().to_string(),
                Field::Subject => state.subject = field.value().to_string(),
            }
        }
    }
    let focused = focus
        .get()
        .and_then(|entity| fields.get(entity).ok().map(|(kind, _)| *kind));
    if let Some(root) = state.root.take() {
        if focus.get().is_some_and(|entity| fields.contains(entity)) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    if changed {
        state.confirmation = None;
        state.pending = None;
        if state.generation != owner.generation() || bound.is_none() {
            state.reply.clear();
            state.subject.clear();
        }
    }
    state.generation = owner.generation();
    state.bound = bound;
    state.dirty = false;
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    if let Some(bound) = &state.bound {
        let (root, field) = widgets::build(&mut commands, &skin, &state, bound, focused);
        state.root = Some(root);
        if let Some(field) = field {
            focus.set(field, bevy::input_focus::FocusCause::Navigated);
        }
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    if let Ok(control) = controls.get(event.entity)
        && control.revision == state.revision
        && state.generation == owner.generation()
        && state.pending.is_none()
    {
        state.pending = Some((control.revision, control.kind));
    }
}
fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
) {
    let Some((revision, kind)) = state.pending.take() else {
        return;
    };
    let (Some(root), Some(bound), Some(play)) =
        (state.root, state.bound.as_ref(), owner.play.as_ref())
    else {
        return;
    };
    if revision != state.revision
        || state.generation != owner.generation()
        || owner.modal_active()
        || !guard.allows(root)
        || play.view.gameplay_input_blocked(&play.link)
        || !bound.same_view(&play.link.cast_chat_bound())
    {
        return;
    }
    for (field_kind, field) in &fields {
        if field.pending_paste.is_some()
            || field
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
        {
            state.pending = Some((revision, kind));
            return;
        }
        match field_kind {
            Field::Reply => state.reply = field.value().to_string(),
            Field::Subject => state.subject = field.value().to_string(),
        }
    }
    match kind {
        Kind::Close => {
            if let Some(bound) = &state.bound {
                let _ = play.link.cast_chat_close_bound(bound);
            }
        }
        Kind::Cancel => {
            state.confirmation = None;
            state.dirty = true;
        }
        Kind::Send(action) if action != 0 => {
            if state
                .bound
                .as_ref()
                .is_some_and(|bound| !bound.panel.pending && bound.panel.selected)
            {
                state.confirmation = Some(Confirmation {
                    text: state.reply.clone(),
                    subject: state.subject.clone(),
                    action,
                });
                state.dirty = true;
            }
        }
        Kind::Send(action) => {
            if let Some(bound) = &state.bound {
                let _ = play.link.cast_chat_submit_bound(
                    bound,
                    &state.reply,
                    &state.subject,
                    action,
                    false,
                );
            }
        }
        Kind::Confirm => {
            if let (Some(bound), Some(confirm)) = (&state.bound, &state.confirmation) {
                let _ = play.link.cast_chat_submit_bound(
                    bound,
                    &confirm.text,
                    &confirm.subject,
                    confirm.action,
                    true,
                );
            }
            state.confirmation = None;
            state.dirty = true;
        }
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Some(root) = state.root else {
        events.clear();
        return;
    };
    if !guard.allows(root) || owner.modal_active() {
        events.clear();
        return;
    }
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) {
        state.pending = Some((
            state.revision,
            if state.confirmation.is_some() {
                Kind::Cancel
            } else {
                Kind::Close
            },
        ));
    }
}
