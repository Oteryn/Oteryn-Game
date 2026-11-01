use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::player_trade::{PlayerTradeResponse, player_trade};
impl Outbox {
    pub(in crate::play) fn player_trade_initialize(
        &self,
        selected: bool,
        state: Option<player_trade::State>,
    ) {
        let mut mailbox = lock(&self.0);
        mailbox.player_trade.selected = selected;
        mailbox.player_trade.state = state;
    }
    pub(in crate::play) fn player_trade_result(&self, response: &PlayerTradeResponse) {
        oteryn_client_ui::player_trade::apply_result(&mut lock(&self.0).player_trade, response);
    }
}
