//! Nonmodal Party List dock; every membership change stays in the canonical party owner.
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    modal::ModalGuard,
};
use crate::play::feature_0042_party::view::PartyViewAction;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::party::{Intent, PartyPanel};
use oteryn_client_ui_bevy as source;
use oteryn_renderer::TileCoord;
use oteryn_session::EntityRef;
#[cfg(test)]
mod tests;
mod widgets;
/// Dedicated 176px sidebar anchor; only this module owns its children.
#[derive(Component)]
pub(super) struct PartyDock;
#[derive(Clone)]
pub(in crate::bevy_shell) struct Target {
    generation: u64,
    tile: TileCoord,
    entity: EntityRef,
    name: Option<String>,
    snapshot: PartyPanel,
}
impl Target {
    pub(in crate::bevy_shell) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    pub(in crate::bevy_shell) fn invite_enabled(&self) -> bool {
        invite_allowed(&self.snapshot)
    }
}
fn invite_allowed(panel: &PartyPanel) -> bool {
    panel.selected
        && panel.pending.is_none()
        && panel
            .state
            .as_ref()
            .is_some_and(|state| state.members.is_empty() || panel.is_leader())
}
fn available(owner: &HostOwner) -> bool {
    owner.phase == Phase::Playing
        && !owner.modal_active()
        && owner
            .play
            .as_ref()
            .is_some_and(|play| !play.view.gameplay_input_blocked(&play.link))
}
pub(in crate::bevy_shell) fn context(owner: &HostOwner, tile: TileCoord) -> Option<Target> {
    if !available(owner) {
        return None;
    }
    let play = owner.play.as_ref()?;
    let snapshot = play.link.party_panel();
    if !snapshot.selected {
        return None;
    }
    let (entity, name) = play.view.party_context_target(tile)?;
    Some(Target {
        generation: owner.generation(),
        tile,
        entity,
        name,
        snapshot,
    })
}
pub(in crate::bevy_shell) fn invite(owner: &HostOwner, target: &Target) -> bool {
    if !available(owner) || target.generation != owner.generation() || !target.invite_enabled() {
        return false;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    if play.view.party_target(target.tile) != Some(target.entity) {
        return false;
    }
    play.link.party_action_bound(
        &target.snapshot,
        PartyViewAction::Request(Intent::Invite(target.entity)),
    )
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    serial: u64,
    snapshot: Option<PartyPanel>,
    root: Option<Entity>,
}
#[derive(Clone, Copy, Component)]
struct Control {
    generation: u64,
    serial: u64,
    kind: Kind,
}
#[derive(Clone, Copy)]
enum Kind {
    Close,
    Refresh,
    Member(u64),
    Invitation(u64),
    Join,
    Leave,
    Pass,
    Kick,
    Shared,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    docks: Query<Entity, With<PartyDock>>,
    mut state: ResMut<State>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|play| play.link.party_is_open())
        .map(|play| play.link.party_panel());
    if state.generation == owner.generation() && state.snapshot == panel {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.generation = owner.generation();
    state.serial = state.serial.saturating_add(1);
    state.snapshot = panel;
    let (Some(panel), Ok(dock)) = (&state.snapshot, docks.single()) else {
        return;
    };
    let root = widgets::build(
        &mut commands,
        &skins,
        dock,
        owner.generation(),
        state.serial,
        panel,
    );
    state.root = Some(root);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) || !available(&owner) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation != owner.generation() || control.serial != state.serial {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if matches!(control.kind, Kind::Close) {
        play.link.party_close();
        return;
    }
    let Some(panel) = &state.snapshot else {
        return;
    };
    let action = match control.kind {
        Kind::Close => return,
        Kind::Refresh => PartyViewAction::Request(Intent::Refresh),
        Kind::Member(handle) => PartyViewAction::Member(handle),
        Kind::Invitation(handle) => PartyViewAction::Invitation(handle),
        Kind::Join => PartyViewAction::Request(Intent::Join),
        Kind::Leave => PartyViewAction::Request(Intent::Leave),
        Kind::Pass => PartyViewAction::Request(Intent::PassLeadership),
        Kind::Kick => PartyViewAction::Request(Intent::Kick),
        Kind::Shared => PartyViewAction::Request(Intent::ToggleSharedExperience),
    };
    play.link.party_action_bound(panel, action);
}
