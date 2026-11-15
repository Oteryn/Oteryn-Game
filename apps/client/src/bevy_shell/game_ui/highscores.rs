//! Highscores presentation over the retained snapshot and request owner.
#[cfg(test)]
mod live_tests;
mod model;
#[cfg(test)]
mod tests;
mod widgets;
use super::{GameModal, modal::ModalGuard};
use crate::bevy_shell::{owner::HostOwner, ui::Skins};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use model::{request_action, table_rows, updated_text};
use oteryn_client_ui::highscores::{Action as Request, HighscoresPanel};
use oteryn_client_ui_bevy as source;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Category,
    Close,
    First,
    Last,
    Next,
    Own,
    Previous,
    Refresh,
    Vocation,
    World(Option<oteryn_client_runtime::highscores::wire::WorldId>),
    WorldInput,
}
#[derive(Component, Clone, Copy)]
struct Action {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct WorldField;
#[derive(Component)]
struct Updated;
#[derive(Resource, Default)]
struct State {
    generation: Option<u64>,
    revision: u64,
    seen: Option<HighscoresPanel>,
    worlds: Vec<(oteryn_client_runtime::highscores::wire::WorldId, String)>,
    frame: Option<Entity>,
    body: Option<Entity>,
    seek: Option<Kind>,
    pending: Option<(Action, Entity)>,
    feedback: Option<String>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
fn setup(mut commands: Commands) {
    commands.spawn((
        Overlay,
        GameModal,
        GlobalZIndex(158),
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
    ));
}
#[allow(clippy::too_many_arguments)] // Projection owns hierarchy, focus and retained session state together.
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut overlay: Query<(Entity, &mut Node), With<Overlay>>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(), With<WorldField>>,
    mut updated: Query<&mut Text, With<Updated>>,
) {
    let Ok((overlay, mut node)) = overlay.single_mut() else {
        return;
    };
    let generation = owner.generation();
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.controls.highscores_generation == Some(generation))
        .map(|play| play.link.highscores_panel());
    if state.generation != Some(generation) || panel.is_none() {
        if let Some(frame) = state.frame.take() {
            commands.entity(frame).despawn();
        }
        if focus.get().is_some_and(|entity| fields.contains(entity)) {
            focus.clear();
        }
        *state = State {
            generation: Some(generation),
            ..default()
        };
    }
    let Some(panel) = panel else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    if let Some(kind) = state.seek
        && !panel.pending
    {
        if panel.feedback.is_none()
            && let Some(action) = request_action(&panel, kind)
            && let Some(play) = &owner.play
        {
            play.link.highscores_request(action);
        } else {
            state.seek = None;
        }
    }
    if state.frame.is_none() {
        state.worlds = owner
            .selection
            .known_worlds()
            .iter()
            .filter_map(|(id, name)| {
                oteryn_client_ui::highscores::parse_world(id)
                    .flatten()
                    .map(|id| (id, name.clone()))
            })
            .collect();
    }
    let now = now_millis();
    if state.seen.as_ref() == Some(&panel) {
        let value = updated_text(&panel, now);
        for mut text in &mut updated {
            if text.0 != value {
                text.0.clone_from(&value);
            }
        }
        return;
    }
    if focus.get().is_some_and(|entity| fields.contains(entity)) {
        focus.clear();
    }
    state.revision = state.revision.wrapping_add(1);
    state.pending = None;
    let action = Action {
        generation,
        revision: state.revision,
        kind: Kind::Close,
    };
    let frame = *state
        .frame
        .get_or_insert_with(|| widgets::frame(&mut commands, &skin, overlay));
    if let Some(body) = state.body.take() {
        commands.entity(body).despawn();
    }
    state.body = Some(widgets::body(
        &mut commands,
        &skin,
        frame,
        action,
        &panel,
        &state.worlds,
        state.feedback.as_deref(),
        now,
    ));
    state.seen = Some(panel);
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if guard.allows(event.entity)
        && let Ok(action) = actions.get(event.entity)
    {
        state.pending = Some((*action, event.entity));
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    focus: Res<InputFocus>,
    fields: Query<(), With<WorldField>>,
    guard: ModalGuard,
) {
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        let Some(frame) = state.frame.filter(|frame| guard.allows(*frame)) else {
            continue;
        };
        let kind = match event.key_code {
            KeyCode::Escape => Kind::Close,
            KeyCode::Enter | KeyCode::NumpadEnter
                if focus.get().is_some_and(|entity| fields.contains(entity)) =>
            {
                Kind::WorldInput
            }
            _ => continue,
        };
        state.pending = Some((
            Action {
                generation: owner.generation(),
                revision: state.revision,
                kind,
            },
            frame,
        ));
    }
}
fn finish(
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<&EditableText, With<WorldField>>,
    guard: ModalGuard,
) {
    let Some((action, entity)) = state.pending.take() else {
        return;
    };
    if !guard.allows(entity)
        || owner.modal_active()
        || action.generation != owner.generation()
        || state.generation != Some(action.generation)
        || action.revision != state.revision
        || owner.controls.highscores_generation != Some(action.generation)
    {
        return;
    }
    if action.kind == Kind::Close {
        owner.controls.highscores_generation = None;
        state.seek = None;
        return;
    }
    let Some(play) = &owner.play else { return };
    let panel = play.link.highscores_panel();
    if state.seen.as_ref() != Some(&panel) {
        return;
    }
    let kind = if action.kind == Kind::WorldInput {
        let Ok(text) = fields.single() else { return };
        if text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
        {
            state.pending = Some((action, entity));
            return;
        }
        let Some(world) = oteryn_client_ui::highscores::parse_world(&text.value().to_string())
        else {
            state.feedback =
                Some("Enter a valid World ID, or leave blank for the current World".into());
            state.seen = None;
            return;
        };
        Kind::World(world)
    } else {
        action.kind
    };
    if let Some(request) = request_action(&panel, kind) {
        state.seek = matches!(kind, Kind::First | Kind::Last).then_some(kind);
        state.feedback = None;
        play.link.highscores_request(request);
    }
}
fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}
