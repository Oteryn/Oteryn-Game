//! Boss reward chest presentation over the retained reward owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    modal::ModalGuard,
};
use crate::play::feature_0224_boss_rewards::view::{
    BossRewardChestAction as Action, BossRewardChestBound as Bound,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::boss_rewards::{BossRewardsPanel as Panel, Intent};
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Request(Intent),
    Confirm,
    Cancel,
    Close,
}
#[derive(Component)]
struct Control {
    revision: u64,
    kind: Kind,
}
#[derive(Component)]
enum Field {
    Death,
    Ordinal,
}
#[derive(Clone)]
struct Snapshot {
    generation: u64,
    bound: Bound,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<Snapshot>,
    revision: u64,
    root: Option<Entity>,
    death: String,
    ordinal: String,
    confirmation: Option<(Intent, String, String)>,
    dirty: bool,
    feedback: String,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn available(owner: &HostOwner) -> bool {
    owner.phase == Phase::Playing
        && !owner.modal_active()
        && owner
            .play
            .as_ref()
            .is_some_and(|p| !p.view.gameplay_input_blocked(&p.link))
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if available(owner)
        && let Some(play) = &owner.play
    {
        let bound = play.link.boss_reward_chest_bound();
        play.link
            .boss_reward_chest_dispatch_bound(&bound, Action::Toggle);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    state: Res<State>,
) {
    for event in events
        .read()
        .filter(|e| e.state == bevy::input::ButtonState::Pressed && !e.repeat)
    {
        if event.key_code == KeyCode::KeyA
            && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
            && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
            && !keys.pressed(KeyCode::AltLeft)
            && !keys.pressed(KeyCode::AltRight)
            && !keys.pressed(KeyCode::SuperLeft)
            && !keys.pressed(KeyCode::SuperRight)
            && guard.allows_focus(state.root)
        {
            open(&mut owner);
        }
        if event.key_code == KeyCode::Escape
            && let Some(root) = state.root
            && guard.allows(root)
            && available(&owner)
            && let Some(snapshot) = &state.snapshot
            && snapshot.generation == owner.generation()
            && let Some(play) = &owner.play
        {
            play.link
                .boss_reward_chest_dispatch_bound(&snapshot.bound, Action::Close);
        }
    }
}
fn edited(fields: &Query<(&Field, &EditableText)>) -> Option<(String, String)> {
    let mut death = None;
    let mut ordinal = None;
    for (field, text) in fields {
        if text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
        {
            return None;
        }
        match field {
            Field::Death => death = Some(text.value().to_string()),
            Field::Ordinal => ordinal = Some(text.value().to_string()),
        }
    }
    Some((death?, ordinal?))
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
    let bound = owner
        .play
        .as_ref()
        .map(|p| p.link.boss_reward_chest_bound());
    let Some(bound) = bound.filter(|b| b.live && b.panel.open) else {
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
    let same = state
        .snapshot
        .as_ref()
        .is_some_and(|s| s.generation == owner.generation() && s.bound.matches(&bound));
    if let Some((death, ordinal)) = edited(&fields) {
        if state
            .confirmation
            .as_ref()
            .is_some_and(|(_, d, o)| *d != death || *o != ordinal)
        {
            state.confirmation = None;
            state.dirty = true;
        }
        state.death = death;
        state.ordinal = ordinal;
    }
    if same && !state.dirty {
        return;
    }
    let same_session = state
        .snapshot
        .as_ref()
        .is_some_and(|s| s.generation == owner.generation() && s.bound.same_session(&bound));
    if !same_session {
        state.death = if bound.panel.death == [0; 16] {
            String::new()
        } else {
            hex(bound.panel.death)
        };
        state.ordinal = bound.panel.selected_ordinal.max(1).to_string();
    }
    if !same {
        state.confirmation = None;
        state.feedback.clear();
    }
    if let Some(root) = state.root.take() {
        if inside(focus.get(), root, &parents) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    state.dirty = false;
    state.root = Some(widgets::build(&mut commands, &skins, &state, &bound.panel));
    state.snapshot = Some(Snapshot {
        generation: owner.generation(),
        bound,
    });
}
fn hex(death: [u8; 16]) -> String {
    death.iter().map(|b| format!("{b:02x}")).collect()
}
fn parse_death(value: &str) -> Option<[u8; 16]> {
    let value = value.trim();
    if value.len() != 32 || !value.is_ascii() {
        return None;
    }
    let mut death = [0; 16];
    for (slot, pair) in death.iter_mut().zip(value.as_bytes().as_chunks::<2>().0) {
        *slot = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    (death != [0; 16]).then_some(death)
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
    let Some(snapshot) = state.snapshot.clone() else {
        return;
    };
    if !guard.allows(event.entity)
        || !available(&owner)
        || control.revision != state.revision
        || snapshot.generation != owner.generation()
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if !play.link.boss_reward_chest_bound().matches(&snapshot.bound) {
        return;
    }
    if control.kind == Kind::Close {
        play.link
            .boss_reward_chest_dispatch_bound(&snapshot.bound, Action::Close);
        return;
    }
    if control.kind == Kind::Cancel {
        state.confirmation = None;
        state.dirty = true;
        return;
    }
    if snapshot.bound.panel.pending || !snapshot.bound.panel.selected {
        return;
    }
    let Some((death_text, ordinal_text)) = edited(&fields) else {
        return;
    };
    let (intent, confirmed) = match control.kind {
        Kind::Request(intent) => (intent, false),
        Kind::Confirm => {
            let Some((intent, death, ordinal)) = state.confirmation.take() else {
                return;
            };
            state.dirty = true;
            if death != death_text || ordinal != ordinal_text {
                return;
            }
            (intent, true)
        }
        Kind::Cancel | Kind::Close => return,
    };
    let death = parse_death(&death_text);
    let ordinal = ordinal_text.trim().parse::<u16>().ok();
    if intent != Intent::CollectAll && death.is_none()
        || intent == Intent::Take && !ordinal.is_some_and(|n| (1..=1000).contains(&n))
    {
        state.feedback = "Enter a valid nonzero death id and reward ordinal.".into();
        state.dirty = true;
        return;
    }
    if intent != Intent::Browse && !confirmed {
        state.confirmation = Some((intent, death_text, ordinal_text));
        state.feedback.clear();
        state.dirty = true;
        return;
    }
    play.link.boss_reward_chest_dispatch_bound(
        &snapshot.bound,
        Action::Request {
            intent,
            death: death.unwrap_or([0; 16]),
            ordinal: ordinal.unwrap_or(0),
            confirmed,
        },
    );
}
