//! Join Team over the negotiated Team Finder member stream.
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
use crate::play::feature_0232_team_finder_members::input::TeamControl as Kind;
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::team_finder::Panel;
#[derive(Clone, Copy, Default, PartialEq, Eq)]
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
enum Field {
    Name,
    Level,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, Panel)>,
    root: Option<Entity>,
    key: Key,
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
    parents: Query<&ChildOf>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| play.link.discovery_panel().teams)
        .filter(|panel| panel.open);
    let snapshot = panel.map(|panel| (owner.generation(), panel));
    if state.snapshot == snapshot {
        return;
    }
    let Some(revision) = state.key.revision.checked_add(1) else {
        return;
    };
    state.key = Key {
        generation: owner.generation(),
        revision,
    };
    if let Some(root) = state.root.take() {
        let mut focused = focus.get();
        while let Some(entity) = focused {
            if entity == root {
                focus.clear();
                break;
            }
            focused = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(root).despawn();
    }
    if let Some((_, panel)) = &snapshot {
        state.root = Some(widgets::build(&mut commands, &skins, state.key, panel));
    }
    state.snapshot = snapshot;
}
fn perform(owner: &HostOwner, state: &State, control: &Control) {
    let Some((generation, expected)) = &state.snapshot else {
        return;
    };
    if owner.phase != Phase::Playing
        || owner.modal_active()
        || owner.generation() != *generation
        || control.key != state.key
    {
        return;
    }
    if let Some(play) = &owner.play
        && (control.kind == Kind::Close || !play.view.gameplay_input_blocked(&play.link))
    {
        play.link
            .team_dispatch_bound(expected, control.kind.clone());
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        let mut control = control.clone();
        if let Kind::Filter(ref mut filter) = control.kind {
            for (field, text) in &fields {
                let value = text.value().to_string();
                match field {
                    Field::Name => filter.name = value,
                    Field::Level => {
                        filter.level = if value.trim().is_empty() {
                            None
                        } else {
                            let Ok(level) = value.trim().parse::<u16>() else {
                                return;
                            };
                            Some(level)
                        };
                    }
                }
            }
        }
        perform(&owner, &state, &control);
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
    if guard.allows(root)
        && events.read().any(|event| {
            event.key_code == KeyCode::Escape
                && event.state == bevy::input::ButtonState::Pressed
                && !event.repeat
        })
    {
        perform(
            &owner,
            &state,
            &Control {
                key: state.key,
                kind: Kind::Close,
            },
        );
    }
}
