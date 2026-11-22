//! Sandbox coin checkout and status. The client never verifies provider events or grants coins.
mod controller;
#[cfg(test)]
pub(crate) mod delivery_tests;
pub(crate) mod launcher;
#[cfg(test)]
mod tests;
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::native_login::coin_purchase::*;
use oteryn_platform_client::native_login::{
    AccessToken, AttemptRef, NativeLoginClient, NativeLoginError,
};
use std::{collections::BTreeMap, sync::Arc, time::Instant};

#[derive(Debug, Clone)]
pub enum Action {
    Refresh,
    Purchase { package_id: String, confirmed: bool },
    Retry,
    Status,
    Inspect(String),
    OpenCheckout,
    Back,
}
#[derive(Debug, Clone, Default)]
pub struct View {
    pub catalogue: Option<CoinCatalogue>,
    pub balance: Option<CoinBalance>,
    pub order: Option<CoinOrder>,
    pub feedback: String,
    pub unavailable: bool,
    character_id: Option<String>,
    pending: Option<CoinOrderRequest>,
    post_refusal_status: Option<u16>,
    checkout_unverified: bool,
    confirmed: Option<CoinPackage>,
    quote_until: BTreeMap<String, Instant>,
}
impl View {
    pub fn new(character_id: Option<String>) -> Self {
        Self {
            character_id,
            feedback: tr!(
                "Choose an Oteryn coin package. Sandbox checkout never charges real money."
            )
            .into(),
            ..Self::default()
        }
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn can_retry(&self) -> bool {
        self.pending() && self.post_refusal_status.is_none()
    }
    pub fn can_purchase(&self, id: &str) -> bool {
        self.can_purchase_at(id, Instant::now())
    }
    pub(crate) fn can_purchase_at(&self, id: &str, now: Instant) -> bool {
        !self.unavailable
            && !self.pending()
            && self.balance.as_ref().is_some_and(|b| !b.review_required)
            && self.catalogue.as_ref().is_some_and(|c| {
                c.packages.iter().any(|p| {
                    p.id == id
                        && self
                            .quote_until
                            .get(&p.quote_id)
                            .is_some_and(|until| *until > now)
                })
            })
    }
    pub fn can_checkout(&self) -> bool {
        !self.checkout_unverified
            && self.order.as_ref().is_some_and(|order| {
                order.state == PurchaseState::AwaitingPayment && order.checkout_url.is_some()
            })
    }
    pub fn status_text(&self) -> String {
        let mut text = self.feedback.clone();
        if let Some(request) = &self.pending {
            text.push_str(&lfmt!(
                "\nUnresolved order request {}: retry this request or check status.",
                request.request_id
            ));
        }
        if let Some(balance) = &self.balance {
            text.push_str(&lfmt!(
                "\nBalance: {} coins.{}",
                balance.available_coins,
                if balance.review_required {
                    tr!(" Account review required; new purchases are disabled.")
                } else {
                    ""
                }
            ));
        }
        if let Some(order) = &self.order {
            text.push_str(&lfmt!(
                "\nOrder {}: {:?}, {} coins for {}. {}",
                order.order_id,
                order.state,
                order.coins,
                price_text(&order.price),
                order.code
            ));
            if let Some(id) = &order.receipt_id {
                text.push_str(&lfmt!("\nReceipt {id}", id = id));
            }
            if order.review_required {
                text.push_str(tr!("\nPayment review required. Contact Oteryn support."));
            }
        }
        text
    }
}
pub fn price_text(price: &Price) -> String {
    if !price.valid() {
        return tr!("Unsupported price").into();
    }
    let divisor = 10_u64.pow(u32::from(price.exponent));
    if price.exponent == 0 {
        format!("{} {}", price.currency, price.minor_units)
    } else {
        format!(
            "{} {}.{:0width$}",
            price.currency,
            price.minor_units / divisor,
            price.minor_units % divisor,
            width = usize::from(price.exponent)
        )
    }
}
pub type Presenter = Arc<dyn Fn(&View, CancellationToken) -> Action + Send + Sync>;
pub type Browser = Arc<dyn Fn(&str) -> bool + Send + Sync>;
pub fn present(view: &View, cancel: CancellationToken) -> Action {
    {
        let _ = (view, cancel);
        Action::Back
    }
}
pub(crate) async fn run(
    client: &NativeLoginClient,
    token: &AccessToken,
    presenter: &Presenter,
    browser: &Browser,
    view: &mut View,
    cancel: CancellationToken,
) -> Result<(), crate::GameplayEntryError> {
    view.catalogue = None;
    controller::execute(
        view,
        Action::Refresh,
        client,
        token,
        browser,
        cancel.clone(),
    )
    .await;
    while !cancel.is_cancelled() {
        let present = presenter.clone();
        let cancellation = cancel.clone();
        let snapshot = view.clone();
        let action = tokio::task::spawn_blocking(move || present(&snapshot, cancellation))
            .await
            .map_err(|_| {
                crate::GameplayEntryError::Rejected(
                    oteryn_platform_client::native_login::PublicClass::TemporarilyUnavailable,
                )
            })?;
        if matches!(action, Action::Back) || cancel.is_cancelled() {
            return Ok(());
        }
        controller::execute(view, action, client, token, browser, cancel.clone()).await;
    }
    Ok(())
}
