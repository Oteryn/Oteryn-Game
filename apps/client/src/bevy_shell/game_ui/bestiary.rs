//! Bestiary presentation over the retained charm progression owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0067_bestiary_charm_progression::view::{
    BestiaryBinding, BestiaryControl as Kind,
};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::charm_progression::{Intent, Panel};
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
    confirm: bool,
}
#[derive(Resource, Default)]
struct State {
    binding: Option<BestiaryBinding>,
    generation: u64,
    revision: u64,
    overlay: Option<Entity>,
    confirmation: Option<Kind>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn presentation(panel: &Panel) -> String {
    format!(
        "{}\n{}\n{}",
        oteryn_client_ui::charm_progression::bestiary_text(panel),
        oteryn_client_ui::charm_progression::charms_text(panel),
        panel.feedback
    )
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    let binding = owner.play.as_ref().map(|p| p.link.bestiary_binding());
    let binding = binding.filter(|b| b.panel.open && b.panel.selected);
    if state.generation == owner.generation()
        && state.binding.as_ref().map(|b| &b.panel) == binding.as_ref().map(|b| &b.panel)
    {
        return;
    }
    state.confirmation = None;
    rebuild(
        &mut commands,
        &skins,
        &mut state,
        binding,
        owner.generation(),
    );
}
fn rebuild(
    commands: &mut Commands,
    skins: &Skins,
    state: &mut State,
    binding: Option<BestiaryBinding>,
    generation: u64,
) {
    if let Some(entity) = state.overlay.take() {
        commands.entity(entity).despawn();
    }
    state.generation = generation;
    state.revision = state.revision.saturating_add(1);
    state.binding = binding;
    if let Some(binding) = &state.binding {
        state.overlay = Some(widgets::build(commands, skins, state, &binding.panel));
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut commands: Commands,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || owner.modal_active()
        || control.generation != owner.generation()
        || control.generation != state.generation
        || control.revision != state.revision
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let Some(binding) = &state.binding else {
        return;
    };
    if control.kind != Kind::Close && !play.link.bestiary_binding_current(binding) {
        return;
    }
    if matches!(control.kind, Kind::Request(Intent::Unlock | Intent::Remove)) && !control.confirm {
        state.confirmation = Some(control.kind);
        let binding = state.binding.clone();
        rebuild(
            &mut commands,
            &skins,
            &mut state,
            binding,
            owner.generation(),
        );
        return;
    }
    if control.confirm && state.confirmation != Some(control.kind) {
        return;
    }
    if play
        .link
        .bestiary_dispatch_bound(binding, control.kind, control.confirm)
    {
        state.confirmation = None;
        let binding = play.link.bestiary_binding();
        let binding = binding.panel.open.then_some(binding);
        rebuild(
            &mut commands,
            &skins,
            &mut state,
            binding,
            owner.generation(),
        );
    }
}

fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) && state.generation == owner.generation()
        && state.overlay.is_some_and(|root| guard.allows(root))
        && let Some(binding) = &state.binding
        && let Some(play) = &owner.play
    {
        play.link
            .bestiary_dispatch_bound(binding, Kind::Close, false);
    }
}
