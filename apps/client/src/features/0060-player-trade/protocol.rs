use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::player_trade::{PlayerTradeResponse, TradeItems, player_trade};
use oteryn_client_ui::player_trade::PlayerTradePanel;
use oteryn_session::SessionEvent;
impl Outbox {
    pub(in crate::play) fn player_trade_initialize(
        &self,
        selected: bool,
        state: Option<player_trade::State>,
    ) {
        let mut mailbox = lock(&self.0);
        mailbox.player_trade.selected = selected;
        mailbox.player_trade.show_requested = state
            .as_ref()
            .is_some_and(|state| matches!(state.phase, 1..=3));
        mailbox.player_trade.partner = state.as_ref().and_then(|state| state.partner);
        mailbox.player_trade.state = state;
    }
    pub(in crate::play) fn player_trade_items(&self, items: TradeItems) {
        lock(&self.0).player_trade.items = items;
    }
    pub(in crate::play) fn player_trade_result(&self, response: &PlayerTradeResponse) {
        oteryn_client_ui::player_trade::apply_result(&mut lock(&self.0).player_trade, response);
    }
    pub(in crate::play) fn player_trade_container_result(
        &self,
        response: &oteryn_session::GatedCommandResult,
    ) {
        use oteryn_session::container_tree::{
            ContainerViewOutcome as O, decode_container_view_result,
        };
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.player_trade;
        panel.items.pending = false;
        panel.feedback = Some(match decode_container_view_result(&response.payload) {
            Ok(O::Opened | O::Closed) => tr!("Container updated"),
            Ok(O::TooFar) => tr!("Move closer to the container"),
            Ok(O::NotAContainer) => tr!("This item is not a container"),
            Ok(O::TooManyViews) => tr!("Close a container first"),
            _ => tr!("That container is no longer available"),
        });
    }
}

pub(in crate::play) fn reduce(panel: &mut PlayerTradePanel, events: &[SessionEvent]) {
    for event in events {
        match event {
            SessionEvent::Inventory(update) => panel.items.inventory = update.value.clone(),
            SessionEvent::OpenContainer(update) => panel.items.corpse = update.value.clone(),
            SessionEvent::ContainerViews(update) => panel.items.containers = update.value.clone(),
            SessionEvent::PlayerTrade(update) => {
                if matches!(update.value.phase, 1..=3)
                    && !panel
                        .state
                        .as_ref()
                        .is_some_and(|state| matches!(state.phase, 1..=3))
                {
                    panel.show_requested = true;
                }
                panel.state = Some(update.value.clone());
                panel.feedback = None;
                if matches!(update.value.phase, 1..=3) {
                    panel.partner = update.value.partner;
                } else {
                    panel.offer = None;
                    panel.partner = None;
                }
            }
            _ => {}
        }
    }
}

pub(in crate::play) fn initial_items<S: oteryn_session::SessionStream>(
    session: &oteryn_session::Session<S>,
) -> TradeItems {
    oteryn_client_runtime::player_trade::TradeItems {
        inventory: session.inventory().cloned().unwrap_or_default(),
        corpse: session.open_container().cloned().unwrap_or_default(),
        containers: session.container_views().cloned().unwrap_or_default(),
        can_open: session.is_selected(14),
        pending: false,
    }
}
