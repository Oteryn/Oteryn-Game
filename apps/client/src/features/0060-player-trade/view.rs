use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::player_trade::PlayerTradePanel;
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
