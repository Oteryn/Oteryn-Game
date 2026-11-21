//! Weapon list and level-gated perk grid over the retained proficiency owner.
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
use crate::play::feature_0195_weapon_proficiency::view::ProficiencyAction as Kind;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Key {
    generation: u64,
    serial: u64,
}
#[derive(Component, Clone, Copy)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Progress;
#[derive(Resource, Default)]
struct State {
    key: Key,
    snapshot: Option<WeaponProficiencyPanel>,
    root: Option<widgets::Widgets>,
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
        .and_then(|play| {
            if play.link.weapon_proficiency_is_open() {
                play.link.weapon_proficiency_tick(std::time::Instant::now());
            }
            let panel = play.link.weapon_proficiency_panel();
            panel.open.then_some(panel)
        });
    if state.key.generation == owner.generation() && state.snapshot == panel {
        return;
    }
    if (state.key.generation != owner.generation() || panel.is_none())
        && let Some(widgets) = state.root.take()
    {
        commands.entity(widgets.overlay).despawn();
    }
    state.key = Key {
        generation: owner.generation(),
        serial: state.key.serial.saturating_add(1),
    };
    let key = state.key;
    if let Some(panel) = &panel {
        let widgets = state
            .root
            .get_or_insert_with(|| widgets::build(&mut commands, &skins));
        widgets::render(&mut commands, &skins, widgets, key, panel);
    }
    state.snapshot = panel;
}
fn perform(owner: &HostOwner, state: &State, control: Control) {
    if owner.phase != Phase::Playing || owner.modal_active() || state.key != control.key {
        return;
    }
    if let (Some(play), Some(expected)) = (&owner.play, &state.snapshot)
        && owner.generation() == control.key.generation
        && (matches!(control.kind, Kind::Close) || !play.view.gameplay_input_blocked(&play.link))
    {
        play.link
            .weapon_proficiency_action_bound(expected, control.kind);
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &state, *control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    overlays: Query<Entity, With<Overlay>>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
) {
    for event in events.read() {
        if event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
            && event.key_code == KeyCode::Escape
            && overlays.iter().any(|entity| guard.allows(entity))
        {
            perform(
                &owner,
                &state,
                Control {
                    key: state.key,
                    kind: Kind::Close,
                },
            );
        }
    }
}
