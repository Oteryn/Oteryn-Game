//! Bosstiary presentation over the existing encounter owner. No gameplay state lives here.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0117_boss_encounter::view::{
    BosstiaryControl as Kind, BosstiarySnapshot as Snapshot,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::boss_encounter::{self as boss, Intent};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Component, Clone)]
struct Control {
    key: Key,
    kind: Kind,
    cooldowns: bool,
}
#[derive(Component)]
struct Field(usize);
#[derive(Resource, Default)]
struct State {
    seen: Option<Snapshot>,
    key: Option<Key>,
    revision: u64,
    roots: Vec<Entity>,
    confirmation: Option<Control>,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project, cooldown_text).chain());
}
pub(in crate::bevy_shell) fn toggle(owner: &mut HostOwner, cooldowns: bool) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        let snapshot = play.link.bosstiary_snapshot();
        let _ = play
            .link
            .bosstiary_dispatch_bound(&snapshot, Kind::Toggle(cooldowns), false);
    }
}
fn main_text(snapshot: &Snapshot) -> String {
    let mut text = boss::panel_text(&snapshot.panel);
    if snapshot
        .panel
        .difficulty_windows
        .contains_key(&snapshot.panel.fields[0])
    {
        text.push_str(snapshot.difficulty.feedback.unwrap_or(""));
    }
    text
}
fn elapsed(snapshot: &Snapshot) -> u64 {
    snapshot.panel.cooldown_received_at.map_or(0, |at| {
        u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX)
    })
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
) {
    let snapshot = owner
        .play
        .as_ref()
        .map(|play| play.link.bosstiary_snapshot());
    let generation = owner.generation();
    let changed = snapshot != state.seen || state.key.is_some_and(|k| k.generation != generation);
    if !changed && !state.dirty {
        return;
    }
    if changed {
        state.confirmation = None;
        let Some(revision) = state.revision.checked_add(1) else {
            return;
        };
        state.revision = revision;
        state.key = Some(Key {
            generation,
            revision,
        });
    }
    state.dirty = false;
    for entity in state.roots.drain(..) {
        commands.entity(entity).despawn();
    }
    // Every regenerated input belongs to this snapshot; discard focus on removed controls.
    if state
        .seen
        .as_ref()
        .is_some_and(|s| s.panel.open || s.panel.cooldowns_open)
    {
        focus.clear();
    }
    state.seen = snapshot;
    let (Some(snapshot), Some(key)) = (&state.seen, state.key) else {
        return;
    };
    let roots = widgets::build(
        &mut commands,
        &skins,
        key,
        snapshot,
        state.confirmation.as_ref(),
    );
    state.roots = roots;
}
fn valid(owner: &HostOwner, state: &State, control: &Control) -> bool {
    state.key.is_some_and(|key| key == control.key)
        && owner.generation() == control.key.generation
        && !owner.modal_active()
        && owner.play.as_ref().is_some_and(|play| {
            !play.view.gameplay_input_blocked(&play.link)
                && state
                    .seen
                    .as_ref()
                    .is_some_and(|s| *s == play.link.bosstiary_snapshot())
        })
}
fn perform(owner: &HostOwner, state: &mut State, control: Control, confirmed: bool) -> bool {
    if !valid(owner, state, &control) {
        return false;
    }
    if control.kind.needs_confirmation() && !confirmed {
        state.confirmation = Some(control);
        state.dirty = true;
        return true;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    let Some(snapshot) = &state.seen else {
        return false;
    };
    let accepted = play
        .link
        .bosstiary_dispatch_bound(snapshot, control.kind, confirmed);
    state.confirmation = None;
    state.dirty = true;
    accepted
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
    let mut control = control.clone();
    // Queued edits/paste must settle before a button may consume a field.
    let field_index = match &control.kind {
        Kind::Search(_) => Some(3),
        Kind::Edit(i, _) => Some(*i),
        Kind::Difficulty(_, _) => Some(4),
        _ => None,
    };
    if let Some(index) = field_index {
        let Some((_, text)) = fields.iter().find(|(field, _)| field.0 == index) else {
            return;
        };
        if text.pending_paste.is_some() || !text.pending_edits.is_empty() {
            return;
        }
        let value = text.value().to_string();
        control.kind = match control.kind {
            Kind::Search(_) => Kind::Search(value),
            Kind::Edit(i, _) => Kind::Edit(i, value),
            Kind::Difficulty(intent, _) => {
                let Ok(level) = value.parse() else {
                    return;
                };
                Kind::Difficulty(intent, level)
            }
            _ => return,
        };
    }
    perform(&owner, &mut state, control, false);
}
#[derive(Component)]
struct Confirm(bool);
fn confirm(
    event: On<Activate>,
    choices: Query<&Confirm>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(choice) = choices.get(event.entity) else {
        return;
    };
    if choice.0 {
        if let Some(control) = state.confirmation.clone() {
            perform(&owner, &mut state, control, true);
        }
    } else {
        state.confirmation = None;
        state.dirty = true;
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    for event in events
        .read()
        .filter(|e| e.state == bevy::input::ButtonState::Pressed && !e.repeat)
    {
        if event.key_code == KeyCode::Escape {
            let Some(key) = state.key else {
                continue;
            };
            if let Some(root) = state.roots.last().copied()
                && guard.allows(root)
            {
                if state.confirmation.is_some() {
                    state.confirmation = None;
                    state.dirty = true;
                } else {
                    let cooldowns = state.seen.as_ref().is_some_and(|s| !s.panel.open);
                    perform(
                        &owner,
                        &mut state,
                        Control {
                            key,
                            kind: Kind::Close(cooldowns),
                            cooldowns,
                        },
                        false,
                    );
                }
            }
        }
    }
}

#[derive(Component)]
struct CooldownText;
fn cooldown_text(owner: NonSend<HostOwner>, mut texts: Query<&mut Text, With<CooldownText>>) {
    if let Some(play) = &owner.play {
        let snapshot = play.link.bosstiary_snapshot();
        for mut text in &mut texts {
            text.0 = boss::cooldown_text(&snapshot.panel, elapsed(&snapshot));
        }
    }
}
