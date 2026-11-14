//! Presentation of the retained daily owners; no local reward or protocol state.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::{
    feature_0151_daily_boost::view::DailyBoostBinding,
    feature_0182_daily_reward::view::{DailyRewardBinding, DailyRewardControl},
};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Refresh,
    Claim,
    Details,
    Returner,
    ReturnerRefresh,
    ReturnerClaim,
    Close,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Window {
    Daily,
    Returner,
    Boost,
}
#[derive(Component)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    revision: u64,
    snapshot: Option<Snapshot>,
    root: Option<Entity>,
}
struct Snapshot {
    key: Key,
    window: Window,
    daily: DailyRewardBinding,
    boost: DailyBoostBinding,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
) {
    let next = owner.play.as_ref().and_then(|play| {
        let daily = play.link.daily_reward_binding();
        let boost = play.link.daily_boost_binding();
        let window = if daily.panel.returner_open {
            Window::Returner
        } else if daily.panel.open {
            Window::Daily
        } else if boost.panel.open {
            Window::Boost
        } else {
            return None;
        };
        Some((window, daily, boost))
    });
    if let (Some(previous), Some((window, daily, boost))) = (&state.snapshot, &next)
        && previous.key.generation == owner.generation()
        && previous.window == *window
        && previous.daily.same_projection(daily)
        && previous.boost.same_projection(boost)
    {
        return;
    }
    if let Some(root) = state.root.take() {
        focus.clear();
        commands.entity(root).despawn();
    }
    state.snapshot = None;
    let Some((window, daily, boost)) = next else {
        return;
    };
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    let snapshot = Snapshot {
        key: Key {
            generation: owner.generation(),
            revision,
        },
        window,
        daily,
        boost,
    };
    state.root = Some(widgets::build(&mut commands, &skins, &snapshot));
    // The catalogue is a separate existing owner request, once the wall query completes.
    if window == Window::Daily
        && !snapshot.daily.panel.pending
        && !snapshot.daily.panel.details_attempted
        && let Some(play) = &owner.play
    {
        let _ = play
            .link
            .daily_reward_dispatch_bound(&snapshot.daily, DailyRewardControl::Details);
    }
    state.snapshot = Some(snapshot);
}
fn perform(owner: &HostOwner, state: &State, control: &Control) -> bool {
    let Some(snapshot) = &state.snapshot else {
        return false;
    };
    if control.key != snapshot.key
        || control.key.generation != owner.generation()
        || owner.modal_active()
    {
        return false;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return false;
    }
    let kind = match (snapshot.window, control.kind) {
        (Window::Boost, Kind::Close) => return play.link.daily_boost_close_bound(&snapshot.boost),
        (Window::Daily, Kind::Close) => DailyRewardControl::Close,
        (Window::Returner, Kind::Close) => DailyRewardControl::ReturnerClose,
        (Window::Daily, Kind::Refresh) => DailyRewardControl::Refresh,
        (Window::Daily, Kind::Claim) => DailyRewardControl::Claim,
        (Window::Daily, Kind::Details) => DailyRewardControl::Details,
        (Window::Daily, Kind::Returner) => DailyRewardControl::OpenReturner,
        (Window::Returner, Kind::ReturnerRefresh) => DailyRewardControl::ReturnerRefresh,
        (Window::Returner, Kind::ReturnerClaim) => DailyRewardControl::ReturnerClaim,
        _ => return false,
    };
    play.link.daily_reward_dispatch_bound(&snapshot.daily, kind)
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        let _ = perform(&owner, &state, control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
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
    }) && let Some(snapshot) = &state.snapshot
    {
        let _ = perform(
            &owner,
            &state,
            &Control {
                key: snapshot.key,
                kind: Kind::Close,
            },
        );
    }
}
