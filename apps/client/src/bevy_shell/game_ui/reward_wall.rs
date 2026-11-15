//! Daily reward catalogue and durable history over the retained capability-182 owner.
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
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::daily_reward::{DailyRewardPanel, Intent};
#[derive(Component)]
struct DayCell;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Close,
    Collect,
    History,
    Refresh,
    Wall,
}
#[derive(Clone, Copy, Component)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    revision: u64,
    snapshot: Option<DailyRewardPanel>,
    root: Option<Entity>,
    history: bool,
    rendered_history: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn collectable(panel: &DailyRewardPanel) -> bool {
    panel.selected
        && !panel.pending
        && panel
            .wall
            .is_some_and(|wall| wall.claim_enabled && !wall.claimed)
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
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| {
            let panel = play.link.daily_reward_panel();
            if panel.open && panel.wall.is_some() && !panel.details_attempted && !panel.pending {
                play.link.daily_reward_details_request();
                play.link.daily_reward_panel()
            } else {
                panel
            }
        })
        .filter(|panel| panel.open);
    if state.generation == owner.generation()
        && state.snapshot == panel
        && state.history == state.rendered_history
    {
        return;
    }
    if let Some(root) = state.root.take() {
        if inside(focus.get(), root, &parents) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    if state.generation != owner.generation() || panel.is_none() {
        state.history = false;
    }
    state.generation = owner.generation();
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    state.snapshot = panel;
    state.rendered_history = state.history;
    if let Some(panel) = &state.snapshot {
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            panel,
            state.history,
            Control {
                generation: state.generation,
                revision,
                kind: Kind::Close,
            },
        ));
    }
}
fn perform(owner: &HostOwner, state: &mut State, control: Control) {
    if control.generation != owner.generation()
        || control.revision != state.revision
        || owner.phase != Phase::Playing
        || owner.modal_active()
    {
        return;
    }
    let (Some(play), Some(expected)) = (&owner.play, &state.snapshot) else {
        return;
    };
    if play.link.daily_reward_panel() != *expected {
        return;
    }
    match control.kind {
        Kind::Close => play.link.daily_reward_close(),
        Kind::History => state.history = true,
        Kind::Wall => state.history = false,
        Kind::Refresh | Kind::Collect => {
            if play.view.gameplay_input_blocked(&play.link) {
                return;
            }
            let intent = if control.kind == Kind::Collect {
                if !collectable(expected) {
                    return;
                }
                Intent::Claim
            } else {
                Intent::Refresh
            };
            play.link.daily_reward_request_bound(expected, intent);
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
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &mut state, *control);
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
    if !guard.allows(root) {
        events.clear();
        return;
    }
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) {
        let control = Control {
            generation: state.generation,
            revision: state.revision,
            kind: Kind::Close,
        };
        perform(&owner, &mut state, control);
    }
}
