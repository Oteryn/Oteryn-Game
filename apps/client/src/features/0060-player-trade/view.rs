#[cfg(feature = "bevy-shell")]
#[path = "presentation.rs"]
mod presentation;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::player_trade::PlayerTradePanel;
#[cfg(feature = "bevy-shell")]
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
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::{context_menu, open_window};

#[cfg(windows)]
impl PlayLink {
    pub(in crate::play) fn player_trade_present(&self) {
        let panel = self.player_trade_panel();
        if !self.uses_shared_frontend() && panel.selected && panel.show_requested {
            open_window(self);
        }
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
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
