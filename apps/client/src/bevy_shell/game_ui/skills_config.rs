//! Skills configuration over the retained skills owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0274_loyalty_skills::view::{
    SkillsConfigControl as Kind, SkillsConfigSnapshot as Panel, SkillsNumbers,
};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    ui_widgets::Activate,
};
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Component, Clone)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct Overlay;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, Panel)>,
    revision: u64,
    refresh_elapsed: std::time::Duration,
    widgets: Option<widgets::Widgets>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project, auto_refresh).chain());
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
    let panel = owner
        .play
        .as_ref()
        .map(|play| play.link.skills_config_snapshot());
    let Some(panel) = panel.filter(|panel| panel.open) else {
        if let Some(widgets) = state.widgets.take() {
            if inside(focus.get(), widgets.overlay, &parents) {
                focus.clear();
            }
            commands.entity(widgets.overlay).despawn();
        }
        state.snapshot = None;
        return;
    };
    if state
        .snapshot
        .as_ref()
        .is_some_and(|(key, old)| key.generation == owner.generation() && old == &panel)
    {
        return;
    }
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    let key = Key {
        generation: owner.generation(),
        revision,
    };
    if state
        .snapshot
        .as_ref()
        .is_some_and(|(old, _)| old.generation != key.generation)
        && let Some(widgets) = state.widgets.take()
    {
        if inside(focus.get(), widgets.overlay, &parents) {
            focus.clear();
        }
        commands.entity(widgets.overlay).despawn();
    }
    let widgets = state
        .widgets
        .get_or_insert_with(|| widgets::build(&mut commands, &skins));
    if widgets
        .content
        .is_some_and(|content| inside(focus.get(), content, &parents))
    {
        focus.set(widgets.panel, FocusCause::Navigated);
    }
    widgets::render(&mut commands, &skins, widgets, key, &panel);
    state.snapshot = Some((key, panel));
}
fn perform(owner: &HostOwner, state: &State, control: &Control) {
    let Some((key, expected)) = &state.snapshot else {
        return;
    };
    if control.key != *key || key.generation != owner.generation() || owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if control.kind != Kind::Close && play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let _ = play
        .link
        .skills_config_dispatch_bound(expected, control.kind);
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
        perform(&owner, &state, control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    let events: Vec<_> = events.read().cloned().collect();
    let Some(widgets) = &state.widgets else {
        return;
    };
    if !guard.allows(widgets.panel) {
        return;
    }
    if events.iter().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) && let Some((key, _)) = &state.snapshot
    {
        perform(
            &owner,
            &state,
            &Control {
                key: *key,
                kind: Kind::Close,
            },
        );
    }
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link.skills_config_open();
    }
}

fn auto_refresh(
    owner: NonSend<HostOwner>,
    time: Res<Time<Real>>,
    windows: Query<&Window>,
    guard: ModalGuard,
    mut state: ResMut<State>,
) {
    if owner.phase != crate::bevy_shell::owner::Phase::Playing
        || windows.iter().any(|window| !window.visible)
        || state
            .widgets
            .as_ref()
            .is_none_or(|widgets| !guard.allows(widgets.panel))
    {
        state.refresh_elapsed = std::time::Duration::ZERO;
        return;
    }
    state.refresh_elapsed += time.delta();
    if state.refresh_elapsed < std::time::Duration::from_secs(1) {
        return;
    }
    state.refresh_elapsed = std::time::Duration::ZERO;
    if let Some((key, _)) = &state.snapshot {
        perform(
            &owner,
            &state,
            &Control {
                key: *key,
                kind: Kind::AutoRefresh,
            },
        );
    }
}
