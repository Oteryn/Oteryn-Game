//! Native sandbox coin purchases v1. Only Platform verifies payments and credits the ledger.
use super::account_services::{WireReply, required_option, unique_ids, valid_id, valid_text};
use super::bazaar::MAX_COINS;
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Price {
    pub minor_units: u64,
    pub currency: String,
    pub exponent: u8,
}
impl Price {
    pub fn valid(&self) -> bool {
        (1..=9_999_999_999).contains(&self.minor_units)
            && self.currency.len() == 3
            && self.currency.bytes().all(|b| b.is_ascii_uppercase())
            && self.exponent <= 3
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoinPackage {
    pub id: String,
    pub quote_id: String,
    pub title: String,
    pub coins: u64,
    pub price: Price,
    pub valid_for_seconds: u32,
}
impl CoinPackage {
    fn valid(&self) -> bool {
        valid_id(&self.id)
            && valid_id(&self.quote_id)
            && valid_text(&self.title, 64)
            && (1..=MAX_COINS).contains(&self.coins)
            && self.price.valid()
            && (1..=300).contains(&self.valid_for_seconds)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoinBalance {
    pub version: u32,
    pub revision: u64,
    pub total_coins: u64,
    pub available_coins: u64,
    pub transferable_coins: u64,
    pub review_required: bool,
}
impl WireReply for CoinBalance {
    fn valid(&self) -> bool {
        self.version == 1
            && self.revision > 0
            && self.total_coins <= MAX_COINS
            && self.available_coins <= self.total_coins
            && self.transferable_coins <= self.total_coins
    }
}
/// Browser URL is bearer-adjacent data: never included in Debug or saved settings.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CheckoutLink(String);
impl CheckoutLink {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Debug for CheckoutLink {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("CheckoutLink([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PurchaseState {
    AwaitingPayment,
    Processing,
    Paid,
    Refunded,
    Chargeback,
    Failed,
}
impl PurchaseState {
    pub fn completed(self) -> bool {
        matches!(
            self,
            Self::Paid | Self::Refunded | Self::Chargeback | Self::Failed
        )
    }
    pub fn can_follow(self, old: Self) -> bool {
        self == old
            || match old {
                Self::AwaitingPayment => true,
                Self::Processing => !matches!(self, Self::AwaitingPayment),
                Self::Paid => matches!(self, Self::Refunded | Self::Chargeback),
                Self::Refunded => self == Self::Chargeback,
                Self::Chargeback | Self::Failed => false,
            }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoinOrder {
    pub version: u32,
    pub order_id: String,
    pub request_id: String,
    pub revision: u64,
    pub package_id: String,
    pub quote_id: String,
    #[serde(deserialize_with = "required_option")]
    pub character_id: Option<String>,
    pub coins: u64,
    pub price: Price,
    pub sandbox: bool,
    pub state: PurchaseState,
    #[serde(deserialize_with = "required_option")]
    pub checkout_url: Option<CheckoutLink>,
    #[serde(deserialize_with = "required_option")]
    pub receipt_id: Option<String>,
    pub review_required: bool,
    pub code: String,
}
impl WireReply for CoinOrder {
    fn valid(&self) -> bool {
        self.version == 1
            && self.revision > 0
            && self.sandbox
            && valid_id(&self.order_id)
            && valid_id(&self.request_id)
            && valid_id(&self.package_id)
            && valid_id(&self.quote_id)
            && self.character_id.as_ref().is_none_or(|c| valid_id(c))
            && (1..=MAX_COINS).contains(&self.coins)
            && self.price.valid()
            && valid_text(&self.code, 96)
            && match self.state {
                PurchaseState::AwaitingPayment => {
                    self.checkout_url
                        .as_ref()
                        .is_some_and(|u| valid_text(u.as_str(), 2048))
                        && self.receipt_id.is_none()
                }
                PurchaseState::Processing | PurchaseState::Failed => {
                    self.checkout_url.is_none() && self.receipt_id.is_none()
                }
                PurchaseState::Paid | PurchaseState::Refunded | PurchaseState::Chargeback => {
                    self.checkout_url.is_none()
                        && self.receipt_id.as_ref().is_some_and(|id| valid_id(id))
                }
            }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoinCatalogue {
    pub version: u32,
    pub sandbox: bool,
    #[serde(deserialize_with = "required_option")]
    pub character_id: Option<String>,
    pub packages: Vec<CoinPackage>,
    pub recent_orders: Vec<CoinOrder>,
}
impl WireReply for CoinCatalogue {
    fn valid(&self) -> bool {
        self.version == 1
            && self.sandbox
            && self.character_id.as_ref().is_none_or(|c| valid_id(c))
            && self.packages.len() <= 20
            && self.recent_orders.len() <= 20
            && self.packages.iter().all(CoinPackage::valid)
            && self.recent_orders.iter().all(WireReply::valid)
            && unique_ids(self.packages.iter().map(|p| p.id.as_str()))
            && unique_ids(self.recent_orders.iter().map(|o| o.request_id.as_str()))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoinOrderRequest {
    pub version: u32,
    pub request_id: String,
    pub package_id: String,
    pub quote_id: String,
    #[serde(deserialize_with = "required_option")]
    pub character_id: Option<String>,
}
impl CoinOrderRequest {
    pub fn valid(&self) -> bool {
        self.version == 1
            && valid_id(&self.request_id)
            && valid_id(&self.package_id)
            && valid_id(&self.quote_id)
            && self.character_id.as_ref().is_none_or(|c| valid_id(c))
    }
}
impl NativeLoginClient {
    /// Sandbox v1 accepts only our hosted checkout path, never an arbitrary provider URL.
    pub fn valid_coin_checkout(&self, link: &CheckoutLink, order_id: &str) -> bool {
        let Ok(url) = Url::parse(link.as_str()) else {
            return false;
        };
        valid_id(order_id)
            && url.origin() == self.platform.origin()
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
            && url.query().is_none()
            && url.path() == format!("/native-account/coin-checkout/{order_id}")
    }
    fn validate_coin_order(&self, order: &CoinOrder) -> Result<(), NativeLoginError> {
        if !order.valid()
            || order
                .checkout_url
                .as_ref()
                .is_some_and(|url| !self.valid_coin_checkout(url, &order.order_id))
        {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok(())
    }
    pub async fn coin_catalogue(
        &self,
        token: &AccessToken,
        character_id: Option<&str>,
        cancel: CancellationToken,
    ) -> Result<CoinCatalogue, NativeLoginError> {
        if character_id.is_some_and(|c| !valid_id(c)) {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let mut url = self
            .platform
            .join("/api/v1/native-account/coin-purchases/catalogue")
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        if let Some(id) = character_id {
            url.query_pairs_mut().append_pair("character_id", id);
        }
        let page: CoinCatalogue = self
            .service_request(token, reqwest::Method::GET, url, None, cancel)
            .await?;
        if page.character_id.as_deref() != character_id {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        for order in &page.recent_orders {
            self.validate_coin_order(order)?;
        }
        Ok(page)
    }
    pub async fn coin_balance(
        &self,
        token: &AccessToken,
        cancel: CancellationToken,
    ) -> Result<CoinBalance, NativeLoginError> {
        let url = self
            .platform
            .join("/api/v1/native-account/coin-purchases/balance")
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        self.service_request(token, reqwest::Method::GET, url, None, cancel)
            .await
    }
    pub async fn create_coin_order(
        &self,
        token: &AccessToken,
        request: &CoinOrderRequest,
        cancel: CancellationToken,
    ) -> Result<CoinOrder, NativeLoginError> {
        if !request.valid() {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let url = self
            .platform
            .join("/api/v1/native-account/coin-purchases/orders")
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        let order: CoinOrder = self
            .service_request(
                token,
                reqwest::Method::POST,
                url,
                Some(
                    serde_json::to_value(request)
                        .map_err(|_| PlatformClientError::InvalidPayload)?,
                ),
                cancel,
            )
            .await?;
        self.validate_coin_order(&order)?;
        if order.request_id != request.request_id
            || order.package_id != request.package_id
            || order.quote_id != request.quote_id
            || order.character_id != request.character_id
        {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok(order)
    }
    pub async fn coin_order_status(
        &self,
        token: &AccessToken,
        request_id: &str,
        cancel: CancellationToken,
    ) -> Result<CoinOrder, NativeLoginError> {
        if !valid_id(request_id) {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let url = self
            .platform
            .join(&format!(
                "/api/v1/native-account/coin-purchases/requests/{request_id}"
            ))
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        let order: CoinOrder = self
            .service_request(token, reqwest::Method::GET, url, None, cancel)
            .await?;
        self.validate_coin_order(&order)?;
        if order.request_id != request_id {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok(order)
    }
}
