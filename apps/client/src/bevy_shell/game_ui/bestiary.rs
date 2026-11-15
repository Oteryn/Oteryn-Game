//! Bestiary catalogue and discovery details over the capability-67 owner.
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
use oteryn_client_ui::charm_progression::{Intent, Panel};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Control {
    generation: u64,
    serial: u64,
    kind: Kind,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Catalogue(usize),
    Select(u16),
    Progress,
    Close,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    serial: u64,
    snapshot: Option<Panel>,
    root: Option<Entity>,
    load_catalogue: bool,
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
        .map(|play| play.link.charm_progression_panel())
        .filter(|panel| panel.open);
    let changed_session = state.generation != owner.generation();
    if changed_session || panel.is_none() {
        if let Some(root) = state.root.take() {
            clear_focus(&mut focus, root, &parents);
            commands.entity(root).despawn();
        }
        state.snapshot = None;
        state.load_catalogue = false;
        state.generation = owner.generation();
    }
    let Some(mut panel) = panel else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if state.root.is_none() && state.snapshot.is_none() {
        state.load_catalogue = true;
        if panel.selected && !panel.pending {
            play.link.charm_progression_request(Intent::Query);
        }
        panel = play.link.charm_progression_panel();
    }
    if state.load_catalogue && panel.selected && !panel.pending {
        play.link.bestiary_catalogue_request(0);
        state.load_catalogue = false;
        panel = play.link.charm_progression_panel();
    }
    if state.snapshot.as_ref() == Some(&panel) {
        return;
    }
    if let Some(root) = state.root.take() {
        clear_focus(&mut focus, root, &parents);
        commands.entity(root).despawn();
    }
    state.serial = state.serial.saturating_add(1);
    let control = Control {
        generation: state.generation,
        serial: state.serial,
        kind: Kind::Close,
    };
    state.root = Some(widgets::build(&mut commands, &skins, control, &panel));
    state.snapshot = Some(panel);
}
fn perform(owner: &HostOwner, state: &State, control: Control) {
    if owner.phase != Phase::Playing
        || owner.modal_active()
        || owner.generation() != control.generation
        || state.serial != control.serial
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.charm_progression_panel();
    if !panel.open || state.snapshot.as_ref() != Some(&panel) {
        return;
    }
    if control.kind == Kind::Close {
        play.link.bestiary_close();
        return;
    }
    if panel.pending || !panel.selected || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match control.kind {
        Kind::Catalogue(action) => play.link.bestiary_catalogue_request(action),
        Kind::Progress => play.link.charm_progression_request(Intent::Query),
        Kind::Select(race)
            if panel
                .catalogue
                .page
                .cards
                .iter()
                .any(|card| card.race == race) =>
        {
            play.link.charm_progression_edit(1, race.to_string());
            play.link.bestiary_catalogue_request(2);
        }
        Kind::Select(_) | Kind::Close => {}
    }
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
        perform(&owner, &state, *control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    for event in events.read() {
        if event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
            && state.root.is_some_and(|root| guard.allows(root))
        {
            perform(
                &owner,
                &state,
                Control {
                    generation: state.generation,
                    serial: state.serial,
                    kind: Kind::Close,
                },
            );
        }
    }
}

fn clear_focus(focus: &mut InputFocus, root: Entity, parents: &Query<&ChildOf>) {
    let mut current = focus.get();
    while let Some(entity) = current {
        if entity == root {
            focus.clear();
            return;
        }
        current = parents.get(entity).ok().map(ChildOf::parent);
    }
}
