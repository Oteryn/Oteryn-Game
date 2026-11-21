#[cfg(bevy_shell)]
#[path = "presentation.rs"]
mod presentation;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::player_trade::PlayerTradePanel;
#[cfg(bevy_shell)]
pub use presentation::{
    TradeAction, TradePhase, same_presentation, trade_action_enabled, trade_phase,
    trade_picker_available,
};
impl PlayLink {
    #[must_use]
    pub fn player_trade_panel(&self) -> PlayerTradePanel {
        lock(&self.mailbox).player_trade.clone()
    }
}


#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_trade_initialize(&self, state: Option<oteryn_session::player_trade::State>) {
        self.player_trade_initialize(true, state);
    }
    pub(crate) fn bevy_trade_response(
        &self,
        response: &oteryn_client_runtime::player_trade::PlayerTradeResponse,
    ) {
        self.player_trade_result(response);
    }
}
