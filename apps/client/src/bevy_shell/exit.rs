//! Portable confirmation for the existing accepted AskBeforeExit preference.
use super::{
    game_ui::{GameModal, modal::ModalGuard},
    owner::HostOwner,
    ui::Skins,
};
use bevy::{
    input_focus::{InputFocus, tab_navigation::TabGroup},
    prelude::*,
    ui_widgets::Activate,
    window::{PrimaryWindow, WindowCloseRequested},
};
use oteryn_client_ui_bevy as source;

#[cfg(test)]
mod tests;
mod widgets;

#[derive(Resource, Default)]
struct State {
    next: u64,
    pending: Option<u64>,
    closing: bool,
    rendered: Option<u64>,
    panel: Option<Entity>,
}
#[derive(Component)]
struct Overlay;
#[derive(Component, Clone, Copy)]
struct Reply {
    id: u64,
    confirm: bool,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<WindowCloseRequested>()
        .add_message::<AppExit>()
        .add_systems(Startup, widgets::setup.after(super::ui::setup))
        .add_systems(PreUpdate, (close_requests, widgets::project).chain())
        .add_systems(
            Update,
            (keyboard, widgets::project)
                .chain()
                .before(super::input::route),
        );
}
fn close_requests(
    mut requests: MessageReader<WindowCloseRequested>,
    primary: Query<(), With<PrimaryWindow>>,
    mut state: ResMut<State>,
    mut owner: NonSendMut<HostOwner>,
    mut exit: MessageWriter<AppExit>,
) {
    for request in requests.read() {
        if !primary.contains(request.window) || state.closing {
            continue;
        }
        if !owner.settings.preferences.miscellaneous.confirm_exit
            || owner.phase == super::owner::Phase::Closed
        {
            finish(&mut state, &mut owner, &mut exit);
        } else if state.pending.is_none()
            && let Some(id) = state.next.checked_add(1)
        {
            state.next = id;
            state.pending = Some(id);
            owner.route_input(
                &[oteryn_input_actions::NormalizedInputEvent::DeviceLost],
                true,
            );
        }
    }
}
fn finish(state: &mut State, owner: &mut HostOwner, exit: &mut MessageWriter<AppExit>) {
    if state.closing {
        return;
    }
    state.closing = true;
    state.pending = None;
    owner.close();
    exit.write(AppExit::Success);
}
fn respond(
    reply: Reply,
    state: &mut State,
    owner: &mut HostOwner,
    exit: &mut MessageWriter<AppExit>,
) {
    if state.closing || state.pending != Some(reply.id) {
        return;
    }
    state.pending = None;
    if reply.confirm {
        finish(state, owner, exit);
    }
}
fn activate(
    event: On<Activate>,
    replies: Query<&Reply>,
    guard: ModalGuard,
    mut state: ResMut<State>,
    mut owner: NonSendMut<HostOwner>,
    mut exit: MessageWriter<AppExit>,
) {
    if guard.allows(event.entity)
        && let Ok(reply) = replies.get(event.entity)
    {
        respond(*reply, &mut state, &mut owner, &mut exit);
    }
}
fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    overlays: Query<Entity, With<Overlay>>,
    guard: ModalGuard,
    mut state: ResMut<State>,
    mut owner: NonSendMut<HostOwner>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(id) = state.pending else {
        keys.clear();
        return;
    };
    if !overlays.iter().any(|entity| guard.allows(entity)) {
        keys.clear();
        return;
    }
    if keys.read().any(|key| {
        key.key_code == KeyCode::Escape
            && key.state == bevy::input::ButtonState::Pressed
            && !key.repeat
    }) {
        respond(
            Reply { id, confirm: false },
            &mut state,
            &mut owner,
            &mut exit,
        );
    }
}
