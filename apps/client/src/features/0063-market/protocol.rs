use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::market::MarketResponse;
impl Outbox {
    pub(in crate::play) fn market_initialize(&self, selected: bool) {
        lock(&self.0).market.selected = selected;
    }
    pub(in crate::play) fn market_result(&self, response: &MarketResponse) {
        oteryn_client_ui::market::apply_result(&mut lock(&self.0).market, response);
    }
}
