use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn market_panel(&self) -> oteryn_client_ui::market::MarketPanel {
        lock(&self.mailbox).market.clone()
    }
}
