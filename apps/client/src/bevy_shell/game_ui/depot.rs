//! Depot boxes over capability 37's authoritative, paged owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{GameModal, modal::ModalGuard};
use crate::bevy_shell::{
    owner::{HostOwner, Phase},
    ui::Skins,
};
use crate::play::feature_0037_depot::view::DepotUiAction as Kind;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::depot::DepotPanel;
use oteryn_client_ui_bevy as source;

#[derive(Component, Clone, Copy)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    revision: u64,
    panel: Option<DepotPanel>,
    root: Option<Entity>,
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
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| play.link.depot_panel())
        .filter(|panel| panel.open);
    if state.generation == owner.generation() && state.panel == panel {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.generation = owner.generation();
    state.revision = state.revision.saturating_add(1);
    state.panel = panel;
    if let Some(panel) = &state.panel {
        state.root = Some(widgets::build(&mut commands, &skins, &state, panel));
    }
}
fn perform(owner: &HostOwner, state: &State, control: Control) {
    if control.generation == owner.generation()
        && control.generation == state.generation
        && control.revision == state.revision
        && let Some(panel) = &state.panel
    {
        super::install::depot_action(owner, panel, control.kind);
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<(&Control, Has<bevy::ui::InteractionDisabled>)>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
) {
    if guard.allows(event.entity)
        && let Ok((control, false)) = controls.get(event.entity)
    {
        perform(&owner, &state, *control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
) {
    for event in events.read() {
        if event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
            && event.key_code == KeyCode::Escape
            && state.root.is_some_and(|root| guard.allows(root))
        {
            perform(
                &owner,
                &state,
                Control {
                    generation: state.generation,
                    revision: state.revision,
                    kind: Kind::Close,
                },
            );
        }
    }
}
