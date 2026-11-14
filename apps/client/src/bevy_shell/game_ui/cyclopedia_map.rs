//! Cyclopedia presentation over the retained map, atlas, discovery and analyser owners.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use crate::play::feature_0057_cyclopedia_map::view::bound::{MapControl, MapSnapshot};
use bevy::{prelude::*, ui_widgets::Activate};
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    snapshot: std::sync::Arc<MapSnapshot>,
    action: MapControl,
    confirmed: bool,
}
#[derive(Resource, Default)]
struct State {
    root: Option<Entity>,
    snapshot: Option<(u64, MapSnapshot)>,
    confirmation: Option<MapControl>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link.cyclopedia_map_toggle_bound();
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    let snapshot = owner
        .play
        .as_ref()
        .map(|p| p.link.cyclopedia_map_snapshot())
        .filter(|s| s.panel.open);
    let current = snapshot.map(|s| (owner.generation(), s));
    if state.snapshot == current {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.confirmation = None;
    if let Some((generation, snapshot)) = &current {
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            *generation,
            snapshot,
            None,
        ));
    }
    state.snapshot = current;
}
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy injects observer resources and queries separately"
)]
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut commands: Commands,
    skins: Res<Skins>,
    fields: Query<&bevy::text::EditableText, With<widgets::Donation>>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if owner.modal_active() || owner.generation() != control.generation {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link)
        || play.link.cyclopedia_map_snapshot() != *control.snapshot
    {
        return;
    }
    if control.action.needs_confirmation() && !control.confirmed {
        state.confirmation = Some(control.action.clone());
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            control.generation,
            &control.snapshot,
            state.confirmation.as_ref(),
        ));
        return;
    }
    if control.confirmed && state.confirmation.as_ref() != Some(&control.action) {
        return;
    }
    let action = if matches!(control.action, MapControl::Donation(_)) {
        let Ok(value) = fields.single() else {
            return;
        };
        MapControl::Donation(value.value().to_string())
    } else {
        control.action.clone()
    };
    play.link
        .cyclopedia_map_dispatch_bound(&control.snapshot, action, control.confirmed);
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
        if event.key_code == KeyCode::F8
            && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
            && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
            && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
            && guard.allows_focus(state.root)
        {
            open(&mut owner);
        } else if event.key_code == KeyCode::Escape
            && let Some(root) = state.root
            && guard.allows(root)
            && let Some((generation, snapshot)) = &state.snapshot
            && *generation == owner.generation()
            && let Some(play) = &owner.play
        {
            play.link
                .cyclopedia_map_dispatch_bound(snapshot, MapControl::Close, false);
        }
    }
}
