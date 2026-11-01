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
pub(in crate::play) use native::open_window;
