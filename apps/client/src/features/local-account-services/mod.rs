//! Native account services run outside an admitted character session.
mod controller;
#[cfg(windows)]
mod native;
#[cfg(test)]
pub(crate) mod test_server;
#[cfg(test)]
mod tests;
#[cfg(windows)]
pub(crate) mod window;

use oteryn_foundation::CancellationToken;
use oteryn_platform_client::native_login::bazaar::*;
use oteryn_platform_client::native_login::{AccessToken, AttemptRef, NativeLoginClient};
use std::{sync::Arc, time::Instant};

#[derive(Debug, Clone)]
pub enum Action {
    Premium,
    CoinShop,
    Refresh(AuctionView),
    Next,
    Bid {
        auction_id: String,
        coins: u64,
        confirmed: bool,
    },
    Sell {
        character_id: String,
        minimum_bid: u64,
        duration_hours: u32,
        confirmed: bool,
    },
    Quote {
        character_id: String,
        world_id: String,
    },
    Transfer {
        confirmed: bool,
    },
    Retry,
    Status,
    Receipt(String),
    Back,
}

#[derive(Debug)]
pub struct View {
    pub page: Option<BazaarPage>,
    pub quote: Option<TransferQuote>,
    pub receipt: Option<OperationReceipt>,
    pub feedback: String,
    pub selected_view: AuctionView,
    pending: Option<BazaarRequest>,
    unresolved: std::collections::BTreeSet<String>,
    quote_until: Option<Instant>,
}
impl Default for View {
    fn default() -> Self {
        Self {
            page: None,
            quote: None,
            receipt: None,
            feedback: tr!("Browse Oteryn auctions, sell an offline character, or transfer worlds.")
                .into(),
            selected_view: AuctionView::All,
            pending: None,
            unresolved: Default::default(),
            quote_until: None,
        }
    }
}
impl View {
    pub fn pending(&self) -> bool {
        self.pending.is_some() || !self.unresolved.is_empty()
    }
    pub fn can_retry(&self) -> bool {
        self.pending.is_some()
    }
    fn pending_id(&self) -> Option<&str> {
        self.pending
            .as_ref()
            .map(|r| r.request_id.as_str())
            .or_else(|| self.unresolved.iter().next().map(String::as_str))
    }
    fn install_page(&mut self, page: BazaarPage) {
        for receipt in &page.recent_operations {
            if receipt.state == OperationState::Pending {
                self.unresolved.insert(receipt.request_id.clone());
            } else {
                self.unresolved.remove(&receipt.request_id);
            }
        }
        self.page = Some(page);
    }
    pub fn can_transfer(&self) -> bool {
        !self.pending()
            && self.quote.as_ref().is_some_and(|q| q.blockers.is_empty())
            && self.quote_until.is_some_and(|until| until > Instant::now())
    }
    pub fn transfer_description(&self, quote: &TransferQuote) -> String {
        let character = self
            .page
            .as_ref()
            .and_then(|p| {
                p.owned_characters
                    .iter()
                    .find(|c| c.id == quote.character_id)
            })
            .map_or(quote.character_id.as_str(), |c| c.name.as_str());
        let world = self
            .page
            .as_ref()
            .and_then(|p| p.worlds.iter().find(|w| w.id == quote.destination_world_id))
            .map_or(quote.destination_world_id.as_str(), |w| w.name.as_str());
        lfmt!(
            "Transfer {character} → {world}: {} coins; cooldown {} seconds. {}",
            quote.price_coins,
            quote.cooldown_seconds,
            quote.blockers.join("; "),
            character = character,
            world = world
        )
    }
    pub fn status_text(&self) -> String {
        let mut text = self.feedback.clone();
        if let Some(id) = self.pending_id() {
            text.push_str(&lfmt!(
                "\nUnresolved operation {id}: check status before another action.",
                id = id
            ));
        }
        if let Some(page) = &self.page {
            text.push_str(&lfmt!(
                "\nLast refreshed balance: {} Oteryn coins",
                page.balance
            ));
        }
        if let Some(quote) = &self.quote {
            text.push_str(&format!("\n{}", self.transfer_description(quote)));
        }
        if let Some(receipt) = &self.receipt {
            text.push_str(&lfmt!(
                "\n{:?}: {} — operation {}; charged {} coins; balance after operation {}",
                receipt.state,
                receipt.code,
                receipt.request_id,
                receipt.charged_coins,
                receipt.balance
            ));
            if let Some(id) = &receipt.receipt_id {
                text.push_str(&lfmt!("; receipt {id}", id = id));
            }
        }
        text
    }
}
pub type Presenter = Arc<dyn Fn(&View, CancellationToken) -> Action + Send + Sync>;
pub fn present(view: &View, cancel: CancellationToken) -> Action {
    #[cfg(windows)]
    {
        native::present(view, cancel)
    }
    #[cfg(not(windows))]
    {
        let _ = (view, cancel);
        Action::Back
    }
}

pub(crate) async fn run(
    client: &NativeLoginClient,
    token: &AccessToken,
    config: &crate::NativeLoginConfig,
    cancel: CancellationToken,
) -> Result<(), crate::GameplayEntryError> {
    let mut view = View::default();
    controller::execute(
        &mut view,
        Action::Refresh(AuctionView::All),
        client,
        token,
        cancel.clone(),
    )
    .await;
    loop {
        let (returned, action) = ask(view, &config.account_services, cancel.clone()).await?;
        view = returned;
        if matches!(action, Action::Back) || cancel.is_cancelled() {
            return Ok(());
        }
        if matches!(action, Action::Premium) {
            crate::premium::run(client, token, &config.premium, cancel.clone()).await?;
            continue;
        }
        if matches!(action, Action::CoinShop) {
            crate::coin_shop::run(
                client,
                token,
                &config.coin_shop,
                &config.open_browser,
                None,
                cancel.clone(),
            )
            .await?;
            controller::execute(
                &mut view,
                Action::Refresh(AuctionView::All),
                client,
                token,
                cancel.clone(),
            )
            .await;
            continue;
        }
        controller::execute(&mut view, action, client, token, cancel.clone()).await;
    }
}
pub(crate) async fn ask(
    view: View,
    presenter: &Presenter,
    cancel: CancellationToken,
) -> Result<(View, Action), crate::GameplayEntryError> {
    let present = presenter.clone();
    tokio::task::spawn_blocking(move || {
        let action = present(&view, cancel);
        (view, action)
    })
    .await
    .map_err(|_| {
        crate::GameplayEntryError::Rejected(
            oteryn_platform_client::native_login::PublicClass::TemporarilyUnavailable,
        )
    })
}
