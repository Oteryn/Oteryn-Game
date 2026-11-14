//! Source-shaped, nonmodal player trade sidebar over the canonical trade owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use crate::play::feature_0060_player_trade::view::{
    TradeAction, TradePhase, same_presentation, trade_action_enabled, trade_phase,
    trade_picker_available,
};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::player_trade::{Intent, PlayerTradePanel};
use oteryn_client_ui_bevy as source;

/// Dedicated dock anchor; its children are exclusively owned by this module.
#[derive(Component)]
pub(super) struct TradeDock;
#[derive(Component)]
struct OfferSlot;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Accept,
    Cancel,
    Close,
    Look(bool),
    Select(oteryn_session::ItemHandle),
    Offer,
    Open(oteryn_session::ItemHandle),
}
impl Kind {
    fn action(self) -> TradeAction {
        match self {
            Self::Accept => TradeAction::Request(Intent::Accept),
            Self::Cancel => TradeAction::Request(Intent::Cancel),
            Self::Look(false) => TradeAction::Request(Intent::LookOwn),
            Self::Look(true) => TradeAction::Request(Intent::LookCounter),
            Self::Select(item) => TradeAction::Select(item),
            Self::Offer => TradeAction::Offer,
            Self::Open(item) => TradeAction::Open(item),
            Self::Close => TradeAction::Close,
        }
    }
}
#[derive(Component, Clone, Copy)]
struct Action {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    revision: u64,
    snapshot: Option<PlayerTradePanel>,
    root: Option<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
fn visible(panel: &PlayerTradePanel) -> bool {
    panel.selected && (panel.open || panel.show_requested && trade_phase(panel).active())
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    docks: Query<Entity, With<TradeDock>>,
    mut state: ResMut<State>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|play| play.link.player_trade_panel());
    let generation = owner.generation();
    if generation == state.generation
        && match (&state.snapshot, &panel) {
            (Some(old), Some(new)) => same_presentation(old, new),
            (None, None) => true,
            _ => false,
        }
    {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.generation = generation;
    state.snapshot = panel;
    let Some(next) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = next;
    let Some(panel) = state.snapshot.as_ref().filter(|panel| visible(panel)) else {
        return;
    };
    let Ok(dock) = docks.single() else {
        state.snapshot = None;
        return;
    };
    state.root = Some(widgets::build(
        &mut commands,
        &skins,
        dock,
        generation,
        next,
        panel,
    ));
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    guard: ModalGuard,
    state: Res<State>,
    owner: NonSend<HostOwner>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if action.generation != owner.generation() || action.revision != state.revision {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if action.kind != Kind::Close
        && (owner.modal_active() || play.view.gameplay_input_blocked(&play.link))
    {
        return;
    }
    if let Some(expected) = &state.snapshot {
        let _ = play
            .link
            .player_trade_dispatch_bound(expected, action.kind.action());
    }
}
