//! Read-only details projected from the retained status owners and default presenters.
#[cfg(test)]
mod tests;
mod widgets;
use super::super::{
    owner::{HostOwner, Phase},
    ui::Skins,
};
use super::modal::ModalGuard;
use crate::play::feature_0280_party_effects::protocol::bound::{
    StatusDetailsAction as Action, StatusDetailsPanel as Panel, StatusDetailsTab as Tab,
};
use bevy::{input_focus::InputFocus, prelude::*, ui::ScrollPosition, ui_widgets::Activate};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    revision: u64,
}
#[derive(Component, Clone, Copy)]
struct Control {
    key: Key,
    action: Action,
}
#[derive(Component)]
struct Body;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(Key, Panel)>,
    revision: u64,
    widgets: Option<widgets::Widgets>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn toggle(owner: &HostOwner) {
    if owner.phase != Phase::Playing || owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let panel = play.link.status_details_panel();
    let action = panel
        .active()
        .map_or(Action::Open(Tab::Party), Action::Close);
    let _ = play.link.status_details_dispatch_bound(&panel, action);
}
fn perform(owner: &HostOwner, state: &State, control: &Control) -> bool {
    let Some((key, expected)) = &state.snapshot else {
        return false;
    };
    if control.key != *key
        || key.generation != owner.generation()
        || owner.phase != Phase::Playing
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
    play.link
        .status_details_dispatch_bound(expected, control.action)
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
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        if event.key_code == KeyCode::Escape
            && let Some(widgets) = &state.widgets
            && guard.allows(widgets.root)
            && let Some((key, panel)) = &state.snapshot
            && let Some(tab) = panel.active()
        {
            let _ = perform(
                &owner,
                &state,
                &Control {
                    key: *key,
                    action: Action::Close(tab),
                },
            );
        }
    }
}
// Independent projection, hierarchy, focus and text resources belong to this UI system.
#[allow(clippy::too_many_arguments)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
    mut text: Query<&mut Text, With<Body>>,
    scrolls: Query<&ScrollPosition>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| play.link.status_details_panel());
    let Some(panel) = panel.filter(|panel| panel.active().is_some()) else {
        remove(&mut commands, &mut state, &mut focus, &parents);
        state.snapshot = None;
        return;
    };
    if state
        .snapshot
        .as_ref()
        .is_none_or(|(key, old)| key.generation != owner.generation() || old != &panel)
    {
        let Some(revision) = state.revision.checked_add(1) else {
            return;
        };
        state.revision = revision;
        let key = Key {
            generation: owner.generation(),
            revision,
        };
        let changed_tab = state.snapshot.as_ref().is_none_or(|(old_key, old)| {
            old_key.generation != key.generation || old.active() != panel.active()
        });
        if state
            .snapshot
            .as_ref()
            .is_some_and(|(old, _)| old.generation != key.generation)
        {
            remove(&mut commands, &mut state, &mut focus, &parents);
        }
        let widgets = state
            .widgets
            .get_or_insert_with(|| widgets::build(&mut commands, &skins));
        let scroll = if changed_tab {
            ScrollPosition::default()
        } else {
            widgets
                .body
                .and_then(|body| scrolls.get(body).ok())
                .cloned()
                .unwrap_or_default()
        };
        clear_focus(widgets.root, &mut focus, &parents);
        if let Some(tab) = panel.active() {
            widgets::render(&mut commands, &skins, widgets, key, &panel, tab, scroll);
        }
        state.snapshot = Some((key, panel));
    }
    if let Some((_, panel)) = &state.snapshot
        && let Some(tab) = panel.active()
    {
        let value = panel.text_at(tab, Instant::now()).replace("\r\n", "\n");
        for mut text in &mut text {
            if text.0 != value {
                text.0.clone_from(&value);
            }
        }
    }
}
fn remove(
    commands: &mut Commands,
    state: &mut State,
    focus: &mut InputFocus,
    parents: &Query<&ChildOf>,
) {
    if let Some(widgets) = state.widgets.take() {
        clear_focus(widgets.root, focus, parents);
        commands.entity(widgets.root).despawn();
    }
}
fn clear_focus(root: Entity, focus: &mut InputFocus, parents: &Query<&ChildOf>) {
    let mut current = focus.get();
    while let Some(entity) = current {
        if entity == root {
            focus.clear();
            break;
        }
        current = parents.get(entity).ok().map(ChildOf::parent);
    }
}
