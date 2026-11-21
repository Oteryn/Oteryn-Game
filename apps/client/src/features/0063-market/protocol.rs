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
        let (next, book) = {
            let mut m = lock(&self.0);
            let applied = oteryn_client_ui::market::apply_result(&mut m.market, &response);
            if changed && applied {
                if m.market.view == oteryn_client_ui::market::MarketView::Browse
                    && m.market.details.statistics.is_some()
                {
                    let query = oteryn_client_ui::market::details::Query::Book {
                        definition: m.market.definition,
                        page: m.market.page,
                    };
                    let book = oteryn_client_ui::market::details::request(&mut m.market, query);
                    if book.is_some() {
                        m.market.details.book_has_next = false;
                    }
                    (None, book)
                } else {
                    (
                        oteryn_client_ui::market::request(
                            &mut m.market,
                            oteryn_client_ui::market::Intent::Refresh,
                        ),
                        None,
                    )
                }
            } else {
                (None, None)
            }
        };
        if let Some(book) = book {
            self.market_details_result(&book, &session.feature(&book).await?);
        }
        if let Some(next) = next {
            self.market_result(&session.market(&next).await?);
        }
        Ok(())
    }
}

impl Outbox {
 pub(in crate::play) fn coin_market_result(&self,response:&oteryn_client_runtime::server_mechanic::ServerMechanicResponse){
  oteryn_client_ui::market::coin_result(&mut lock(&self.0).market.coins,response);
 }
}

#[path = "protocol/details.rs"]
pub(in crate::play) mod details;
