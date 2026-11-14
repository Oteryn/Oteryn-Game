//! House move-out projects the capability-59 owner; consent is transient presentation state.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0059_house_move_out::view::bound::{Action, Snapshot};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::house_move_out::{Intent, Panel};
use oteryn_client_ui_bevy as source;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Request(Intent),
    Confirm,
    Back,
    Close,
}
#[derive(Component, Clone, Copy)]
struct Control {
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
    confirmation: Option<(Intent, [String; 2])>,
    dirty: bool,
    feedback: String,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if owner.phase == crate::bevy_shell::owner::Phase::Playing
        && !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link.house_move_out_toggle_bound();
    }
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
        .map(|play| play.link.house_move_out_snapshot())
        .filter(|snapshot| snapshot.panel.open);
    let changed = state.generation != owner.generation() || state.snapshot != snapshot;
    if !changed && !state.dirty {
        return;
    }
    if changed {
        state.confirmation = None;
        state.feedback.clear();
    }
    if let Some(root) = state.root {
        let mut entity = focus.get();
        while let Some(current) = entity {
            if current == root {
                focus.clear();
                break;
            }
            entity = parents.get(current).ok().map(ChildOf::parent);
        }
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.generation = owner.generation();
    state.revision = state.revision.wrapping_add(1);
    state.snapshot = snapshot;
    state.dirty = false;
    if let Some(snapshot) = &state.snapshot {
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            &state,
            &snapshot.panel,
        ));
    }
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
    if !guard.allows(event.entity)
        || control.revision != state.revision
        || state.generation != owner.generation()
        || owner.modal_active()
        || owner.phase != crate::bevy_shell::owner::Phase::Playing
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let Some(snapshot) = &state.snapshot else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link)
        || play.link.house_move_out_snapshot() != *snapshot
    {
        return;
    }
    match control.kind {
        Kind::Close => {
            play.link
                .house_move_out_dispatch_bound(snapshot, Action::Close);
        }
        Kind::Back => {
            state.confirmation = None;
            state.dirty = true;
        }
        Kind::Request(intent) => {
            if !snapshot.panel.selected || snapshot.panel.pending {
                return;
            }
            let mut values = snapshot.panel.fields.clone();
            for (field, text) in &fields {
                // Do not consent to text whose destructive edit/paste has not resolved yet.
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
            if intent == Intent::Query {
                if let Err(reason) = play.link.house_move_out_try_dispatch_bound(
                    snapshot,
                    Action::Request {
                        intent,
                        fields: values,
                        confirmed: false,
                    },
                ) {
                    state.feedback = reason;
                    state.dirty = true;
                }
            } else {
                state.confirmation = Some((intent, values));
                state.dirty = true;
            }
        }
        Kind::Confirm => {
            let Some((intent, values)) = &state.confirmation else {
                return;
            };
            let result = play.link.house_move_out_try_dispatch_bound(
                snapshot,
                Action::Request {
                    intent: *intent,
                    fields: values.clone(),
                    confirmed: true,
                },
            );
            match result {
                Ok(()) => state.confirmation = None,
                Err(reason) => state.feedback = reason,
            }
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
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        if event.key_code == KeyCode::Escape
            && state.root.is_some_and(|root| guard.allows(root))
            && !owner.modal_active()
            && state.generation == owner.generation()
        {
            if state.confirmation.take().is_some() {
                state.dirty = true;
            } else if let (Some(play), Some(snapshot)) = (&owner.play, &state.snapshot) {
                play.link
                    .house_move_out_dispatch_bound(snapshot, Action::Close);
            }
        }
    }
}
