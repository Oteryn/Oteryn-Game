//! Training presentation over the retained capability-54 owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0054_training::view::TrainingBinding;
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::training::Intent;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Close,
    Request(Intent),
    Confirm,
    Cancel,
}
#[derive(Component, Clone, Copy)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Component)]
struct Field(usize);
#[derive(Resource, Default)]
struct State {
    generation: u64,
    revision: u64,
    snapshot: Option<TrainingBinding>,
    root: Option<Entity>,
    confirmation: Option<(Intent, [String; 3])>,
    draft: Option<[String; 3]>,
    dirty: bool,
    queued: Option<(Entity, Control)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
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
        let binding = play.link.training_binding();
        if binding.panel.open {
            play.link
                .training_dispatch_bound(&binding, binding.panel.fields.clone(), None, false);
        } else {
            play.link.training_open();
        }
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
    let binding = owner
        .play
        .as_ref()
        .map(|play| play.link.training_binding())
        .filter(|binding| binding.panel.open);
    let same = state.generation == owner.generation()
        && state.snapshot.as_ref().map(|snapshot| &snapshot.panel)
            == binding.as_ref().map(|snapshot| &snapshot.panel);
    if same && !state.dirty {
        return;
    }
    if !same {
        state.draft = None;
        state.confirmation = None;
        state.queued = None;
    }
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
    state.generation = owner.generation();
    state.revision = state.revision.saturating_add(1);
    state.snapshot = binding;
    state.dirty = false;
    if let Some(binding) = &state.snapshot {
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            &state,
            &binding.panel,
        ));
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && available(&owner)
        && let Ok(control) = controls.get(event.entity)
        && control.generation == owner.generation()
        && control.revision == state.revision
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
        || !available(&owner)
        || control.generation != owner.generation()
        || control.revision != state.revision
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let Some(binding) = &state.snapshot else {
        return;
    };
    if play.link.training_panel() != binding.panel {
        return;
    }
    let mut values = binding.panel.fields.clone();
    if state.confirmation.is_none() && control.kind != Kind::Close {
        let mut seen = [false; 3];
        for (field, text) in &fields {
            if field.0 >= 3 {
                return;
            }
            if text.pending_paste.is_some()
                || text
                    .pending_edits
                    .iter()
                    .any(bevy::text::TextEdit::is_destructive)
            {
                state.queued = Some((entity, control));
                return;
            }
            values[field.0] = text.value().to_string();
            seen[field.0] = true;
        }
        if !seen.into_iter().all(|value| value) {
            return;
        }
    }
    match control.kind {
        Kind::Close => {
            play.link
                .training_dispatch_bound(binding, values, None, false);
        }
        Kind::Cancel => {
            state.draft = state.confirmation.take().map(|(_, fields)| fields);
            state.dirty = true;
        }
        Kind::Request(Intent::Status) => {
            play.link
                .training_dispatch_bound(binding, values, Some(Intent::Status), false);
        }
        Kind::Request(intent) => {
            if binding.panel.selected && !binding.panel.pending {
                state.confirmation = Some((intent, values));
                state.dirty = true;
            }
        }
        Kind::Confirm => {
            if let Some((intent, values)) = state.confirmation.clone() {
                play.link
                    .training_dispatch_bound(binding, values, Some(intent), true);
                state.confirmation = None;
                state.dirty = true;
            }
        }
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    focus: Res<InputFocus>,
) {
    let allowed = state.root.map_or_else(
        || guard.allows_focus(focus.get()),
        |root| guard.allows(root),
    );
    if !allowed || !available(&owner) {
        events.clear();
        return;
    }
    for event in events.read() {
        if event.repeat || event.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        let toggle = event.key_code == KeyCode::KeyN
            && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
            && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
            && !keys.pressed(KeyCode::ShiftLeft)
            && !keys.pressed(KeyCode::ShiftRight)
            && !keys.pressed(KeyCode::SuperLeft)
            && !keys.pressed(KeyCode::SuperRight);
        if toggle || (event.key_code == KeyCode::Escape && state.root.is_some()) {
            state.queued = None;
            open(&owner);
            break;
        }
    }
}
