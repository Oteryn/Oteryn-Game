//! Highscores and character information project the retained session owners.
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use crate::play::PlayLink;
use bevy::{prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::{
    character_info::{self, Intent, Panel},
    highscores::{self, Action, HighscoresPanel},
};
#[cfg(test)]
mod tests;
mod widgets;

#[derive(Clone, PartialEq, Eq)]
enum Snapshot {
    Highscores(HighscoresPanel),
    Character(Panel),
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    serial: u64,
    snapshot: Option<Snapshot>,
    root: Option<Entity>,
    submission: Option<(Entity, Control)>,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    serial: u64,
    kind: Kind,
}
#[derive(Clone)]
enum Kind {
    Close,
    Highscores(Action),
    World,
    Character(Intent),
    Title(u8),
}
#[derive(Component)]
struct Field(usize);

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(
        Update,
        (
            keyboard,
            finish.after(bevy::text::EditableTextSystems),
            project,
        )
            .chain(),
    );
}
/// Registration in the existing shortcut router; opening has no protocol side effects.
pub(in crate::bevy_shell) fn input(
    link: &PlayLink,
    events: &[oteryn_input_actions::NormalizedInputEvent],
) -> bool {
    if highscores::open_key(events) {
        link.highscores_open_bound(&link.highscores_panel());
        true
    } else if character_info::open_key(events) || character_info::titles_key(events) {
        link.character_info_open_bound(&link.character_info_panel());
        true
    } else {
        false
    }
}
fn snapshot(owner: &HostOwner) -> Option<Snapshot> {
    if owner.phase != Phase::Playing {
        return None;
    }
    let play = owner.play.as_ref()?;
    let highscores = play.link.highscores_panel();
    if highscores.open {
        return Some(Snapshot::Highscores(highscores));
    }
    let panel = play.link.character_info_panel();
    (panel.open || panel.titles_open).then_some(Snapshot::Character(panel))
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
) {
    let snapshot = snapshot(&owner);
    if state.generation == owner.generation() && state.snapshot == snapshot {
        return;
    }
    if let Some(root) = state.root.take() {
        focus.clear();
        commands.entity(root).despawn();
    }
    state.submission = None;
    state.generation = owner.generation();
    state.serial = state.serial.saturating_add(1);
    state.snapshot = snapshot;
    if let Some(panel) = &state.snapshot {
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            state.generation,
            state.serial,
            panel,
        ));
    }
}
fn perform(owner: &HostOwner, state: &State, control: &Control, fields: [String; 4]) -> bool {
    if owner.phase != Phase::Playing
        || owner.modal_active()
        || control.generation != owner.generation()
        || control.generation != state.generation
        || control.serial != state.serial
    {
        return false;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return false;
    }
    match (&state.snapshot, &control.kind) {
        (Some(Snapshot::Highscores(panel)), Kind::Close) => play.link.highscores_close_bound(panel),
        (Some(Snapshot::Character(panel)), Kind::Close) => {
            play.link.character_info_close_bound(panel)
        }
        (Some(Snapshot::Highscores(panel)), Kind::Highscores(action)) => {
            play.link.highscores_request_bound(panel, *action)
        }
        (Some(Snapshot::Highscores(panel)), Kind::World) => highscores::parse_world(&fields[0])
            .is_some_and(|world| {
                play.link
                    .highscores_request_bound(panel, Action::World(world))
            }),
        (Some(Snapshot::Character(panel)), Kind::Character(intent)) => play
            .link
            .character_info_request_bound(panel, fields, *intent),
        (Some(Snapshot::Character(panel)), Kind::Title(id)) => {
            let mut fields = panel.fields.clone();
            fields[3] = id.to_string();
            play.link
                .character_info_request_bound(panel, fields, Intent::Select)
        }
        _ => false,
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if state.submission.is_some() || !guard.allows(event.entity) {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        if !matches!(
            control.kind,
            Kind::Close | Kind::Title(_) | Kind::Highscores(_)
        ) && fields.iter().any(|(_, text)| editing(text))
        {
            state.submission = Some((event.entity, control.clone()));
            return;
        }
        let mut values = std::array::from_fn(|_| String::new());
        for (field, value) in &fields {
            values[field.0] = value.value().to_string();
        }
        perform(&owner, &state, control, values);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
) {
    let Some(root) = state.root else {
        events.clear();
        return;
    };
    if !guard.allows(root) {
        events.clear();
        return;
    }
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) {
        perform(
            &owner,
            &state,
            &Control {
                generation: state.generation,
                serial: state.serial,
                kind: Kind::Close,
            },
            std::array::from_fn(|_| String::new()),
        );
    }
}

fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
/// A click may arrive before the editor has applied this frame's final keystroke or paste.
fn finish(
    fields: Query<(&Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    let Some((source, _)) = &state.submission else {
        return;
    };
    if !guard.allows(*source) {
        state.submission = None;
        return;
    }
    if fields.iter().any(|(_, text)| editing(text)) {
        return;
    }
    let Some((_, control)) = state.submission.take() else {
        return;
    };
    let mut values = std::array::from_fn(|_| String::new());
    for (field, value) in &fields {
        values[field.0] = value.value().to_string();
    }
    perform(&owner, &state, &control, values);
}
