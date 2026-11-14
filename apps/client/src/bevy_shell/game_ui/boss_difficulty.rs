//! Context-bound difficulty selection over the retained feature owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::boss_difficulty::{BossDifficultyPanel as Panel, Intent};
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Request(Intent),
    Confirm,
    Back,
    Close,
}
#[derive(Component)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Boss,
    Difficulty,
}
#[derive(Component)]
struct Overlay;
#[derive(Clone)]
struct Confirmation {
    intent: Intent,
    boss: String,
    difficulty: u16,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, Panel)>,
    revision: u64,
    overlay: Option<Entity>,
    confirmation: Option<Confirmation>,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play {
        let expected = play.link.boss_difficulty_panel();
        if !play.view.gameplay_input_blocked(&play.link) {
            let _ = play
                .link
                .boss_difficulty_set_open_bound(&expected, !expected.open);
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
) {
    let panel = owner.play.as_ref().map(|p| p.link.boss_difficulty_panel());
    let Some(panel) = panel.filter(|p| p.open) else {
        if let Some(overlay) = state.overlay.take() {
            commands.entity(overlay).despawn();
            if inside(focus.get(), overlay, &parents) {
                focus.clear();
            }
        }
        state.snapshot = None;
        state.confirmation = None;
        return;
    };
    let same = state
        .snapshot
        .as_ref()
        .is_some_and(|(key, old)| key.generation == owner.generation() && old == &panel);
    if same && !state.dirty {
        return;
    }
    if !same {
        state.confirmation = None;
    }
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    state.dirty = false;
    if let Some(overlay) = state.overlay.take() {
        commands.entity(overlay).despawn();
        if inside(focus.get(), overlay, &parents) {
            focus.clear();
        }
    }
    let key = Key {
        generation: owner.generation(),
        revision,
    };
    state.overlay = Some(widgets::build(
        &mut commands,
        &skins,
        key,
        &panel,
        state.confirmation.as_ref(),
    ));
    state.snapshot = Some((key, panel));
}
fn valid(owner: &HostOwner, state: &State, key: Key) -> bool {
    !state.dirty
        && !owner.modal_active()
        && key.generation == owner.generation()
        && state.snapshot.as_ref().is_some_and(|(current, expected)| {
            *current == key
                && owner.play.as_ref().is_some_and(|play| {
                    !play.view.gameplay_input_blocked(&play.link)
                        && play.link.boss_difficulty_panel() == *expected
                })
        })
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
    if !guard.allows(event.entity) || !valid(&owner, &state, control.key) {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let Some((_, expected)) = state.snapshot.clone() else {
        return;
    };
    match control.kind {
        Kind::Close => {
            let _ = play.link.boss_difficulty_set_open_bound(&expected, false);
        }
        Kind::Back => {
            state.confirmation = None;
            state.dirty = true;
        }
        Kind::Confirm => {
            if let Some(confirmation) = state.confirmation.take() {
                let _ = play.link.boss_difficulty_dispatch_bound(
                    &expected,
                    confirmation.intent,
                    &confirmation.boss,
                    confirmation.difficulty,
                    true,
                );
                state.dirty = true;
            }
        }
        Kind::Request(intent) => {
            if expected.pending || !expected.selected {
                return;
            }
            let mut boss = String::new();
            let mut difficulty = None;
            for (field, text) in &fields {
                // Do not submit text while Bevy still has an uncommitted paste/edit.
                if text.pending_paste.is_some()
                    || text
                        .pending_edits
                        .iter()
                        .any(bevy::text::TextEdit::is_destructive)
                {
                    return;
                }
                match field {
                    Field::Boss => boss = text.value().to_string(),
                    Field::Difficulty => {
                        difficulty = text.value().to_string().trim().parse::<u16>().ok()
                    }
                }
            }
            let difficulty = if matches!(intent, Intent::Open | Intent::Cancel) {
                expected.difficulty
            } else {
                let Some(value) = difficulty
                    .filter(|v| *v <= oteryn_client_runtime::boss_difficulty::MAX_DIFFICULTY)
                else {
                    return;
                };
                value
            };
            if intent != Intent::Open && boss.trim() != expected.boss_key {
                return;
            }
            if matches!(intent, Intent::Start | Intent::Cancel) {
                if expected.context.is_none() {
                    return;
                }
                state.confirmation = Some(Confirmation {
                    intent,
                    boss,
                    difficulty,
                });
                state.dirty = true;
            } else {
                state.dirty = play
                    .link
                    .boss_difficulty_dispatch_bound(&expected, intent, &boss, difficulty, false);
            }
        }
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Some(overlay) = state.overlay else {
        events.clear();
        return;
    };
    let Some((key, expected)) = &state.snapshot else {
        events.clear();
        return;
    };
    if !guard.allows(overlay) || !valid(&owner, &state, *key) {
        events.clear();
        return;
    }
    if events.read().any(|e| {
        (e.key_code == KeyCode::Escape
            || e.key_code == KeyCode::KeyY
                && keys.as_ref().is_some_and(|keys| {
                    keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
                        && !keys.any_pressed([
                            KeyCode::AltLeft,
                            KeyCode::AltRight,
                            KeyCode::ShiftLeft,
                            KeyCode::ShiftRight,
                            KeyCode::SuperLeft,
                            KeyCode::SuperRight,
                        ])
                }))
            && e.state == bevy::input::ButtonState::Pressed
            && !e.repeat
    }) {
        if state.confirmation.is_some() {
            state.confirmation = None;
            state.dirty = true;
        } else if let Some(play) = &owner.play {
            let _ = play.link.boss_difficulty_set_open_bound(expected, false);
        }
    }
}
