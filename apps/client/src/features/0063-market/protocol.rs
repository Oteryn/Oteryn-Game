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

impl Outbox {
    pub(in crate::play) async fn market_execute<S: crate::play::Stepper>(
        &self,
        session: &mut S,
        request: &oteryn_client_runtime::market::MarketCommand,
    ) -> Result<(), oteryn_session::SessionError> {
        let response = session.market(request).await?;
        let changed = response
            .reply
            .as_ref()
            .is_some_and(|r| r.code == 1 && r.action >= 5);
        let next = {
            let mut m = lock(&self.0);
            let applied = oteryn_client_ui::market::apply_result(&mut m.market, &response);
            if changed && applied {
                oteryn_client_ui::market::request(
                    &mut m.market,
                    oteryn_client_ui::market::Intent::Refresh,
                )
            } else {
                None
            }
        };
        if let Some(next) = next {
            self.market_result(&session.market(&next).await?);
        }
        Ok(())
    }
}

#[path = "protocol/details.rs"]
pub(in crate::play) mod details;
