use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::market::MarketResponse;
impl Outbox {
    pub(in crate::play) fn market_initialize(&self, selected: bool) {
        lock(&self.0).market.selected = selected;
    }
    pub(in crate::play) fn market_result(&self, response: &MarketResponse) {
        let mut mailbox = lock(&self.0);
        let source_query = mailbox.market.pending.as_ref().is_some_and(|pending| {
            pending.0.action == 2
                && mailbox.discovery.inspection.catalogue_market_requested
                    == Some(pending.0.definition)
        });
        let accepted = oteryn_client_ui::market::apply_result(&mut mailbox.market, response);
        if source_query && !accepted {
            mailbox.discovery.inspection.catalogue_market_requested = None;
            mailbox.discovery.inspection.feedback = mailbox
                .market
                .feedback
                .unwrap_or("Market query refused")
                .into();
        }
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
