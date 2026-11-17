use super::*;
use oteryn_platform_client::native_login::{AttemptRef, coin_purchase::*};
use std::time::Instant;
mod events;
mod provider;
mod transport;
pub(crate) use provider::{Event, EventKind, PaymentProvider, ProviderError, SandboxProvider};
pub(crate) const PACKAGE: &str = "01890a5d-ac96-774b-bcce-b302099a8060";

pub(crate) struct CoinAuthority {
    pub provider: SandboxProvider,
    pub balance: CoinBalance,
    pub orders: BTreeMap<String, (CoinOrderRequest, CoinOrder)>,
    pub events: BTreeMap<String, (Vec<u8>, Result<(), ProviderError>)>,
    quotes: BTreeMap<String, (CoinPackage, Option<String>, Instant, bool)>,
    pub fail_after_create: bool,
    pub creations: u32,
    pub credits: u32,
    pub reversals: u32,
    pub tamper_checkout: Option<String>,
}
impl CoinAuthority {
    pub fn new(origin: String) -> TestResultWith<Self> {
        Ok(Self {
            provider: SandboxProvider::new(origin)?,
            balance: CoinBalance {
                version: 1,
                revision: 1,
                total_coins: 1000,
                available_coins: 1000,
                transferable_coins: 1000,
                review_required: false,
            },
            orders: BTreeMap::new(),
            events: BTreeMap::new(),
            quotes: BTreeMap::new(),
            fail_after_create: false,
            creations: 0,
            credits: 0,
            reversals: 0,
            tamper_checkout: None,
        })
    }
    pub fn route(&mut self, method: &str, target: &str, body: &str) -> Option<(u16, String)> {
        if method == "GET" && target.starts_with("/api/v1/native-account/coin-purchases/catalogue")
        {
            let character = target
                .split_once("?character_id=")
                .map(|(_, id)| id.to_owned());
            if character.as_ref().is_some_and(|id| id != CHARACTER) {
                return Some((403, String::new()));
            }
            let quote_id = AttemptRef::generate().ok()?.as_str().to_owned();
            let package = CoinPackage {
                id: PACKAGE.into(),
                quote_id: quote_id.clone(),
                title: "Sandbox coins".into(),
                coins: 100,
                price: Price {
                    minor_units: 499,
                    currency: "USD".into(),
                    exponent: 2,
                },
                valid_for_seconds: 120,
            };
            self.quotes.insert(
                quote_id,
                (package.clone(), character.clone(), Instant::now(), false),
            );
            let catalogue = CoinCatalogue {
                version: 1,
                sandbox: true,
                character_id: character,
                packages: vec![package],
                recent_orders: self
                    .orders
                    .values()
                    .take(20)
                    .map(|(_, o)| o.clone())
                    .collect(),
            };
            return Some((200, serde_json::to_string(&catalogue).unwrap_or_default()));
        }
        if method == "GET" && target == "/api/v1/native-account/coin-purchases/balance" {
            return Some((
                200,
                serde_json::to_string(&self.balance).unwrap_or_default(),
            ));
        }
        if method == "POST" && target == "/api/v1/native-account/coin-purchases/orders" {
            let Ok(request) = serde_json::from_str::<CoinOrderRequest>(body) else {
                return Some((400, String::new()));
            };
            if !request.valid() {
                return Some((400, String::new()));
            }
            if let Some((stored, order)) = self.orders.get(&request.request_id) {
                return Some(if stored == &request {
                    (200, serde_json::to_string(order).unwrap_or_default())
                } else {
                    (409, String::new())
                });
            }
            if self.balance.review_required {
                return Some((403, String::new()));
            }
            let Some((package, character, at, used)) = self.quotes.get_mut(&request.quote_id)
            else {
                return Some((409, String::new()));
            };
            if *used
                || at.elapsed().as_secs() >= u64::from(package.valid_for_seconds)
                || package.id != request.package_id
                || character != &request.character_id
            {
                return Some((409, String::new()));
            }
            *used = true;
            let id = AttemptRef::generate().ok()?.as_str().to_owned();
            let url = self
                .tamper_checkout
                .clone()
                .unwrap_or_else(|| self.provider.checkout_url(&id));
            let order = CoinOrder {
                version: 1,
                order_id: id,
                request_id: request.request_id.clone(),
                revision: 1,
                package_id: package.id.clone(),
                quote_id: package.quote_id.clone(),
                character_id: character.clone(),
                coins: package.coins,
                price: package.price.clone(),
                sandbox: true,
                state: PurchaseState::AwaitingPayment,
                checkout_url: serde_json::from_value(serde_json::json!(url)).ok(),
                receipt_id: None,
                review_required: false,
                code: "SANDBOX_AWAITING_PAYMENT".into(),
            };
            self.orders
                .insert(request.request_id.clone(), (request, order.clone()));
            self.creations += 1;
            if self.fail_after_create {
                self.fail_after_create = false;
                return Some((503, String::new()));
            }
            return Some((200, serde_json::to_string(&order).unwrap_or_default()));
        }
        if method == "GET"
            && let Some(id) = target.strip_prefix("/api/v1/native-account/coin-purchases/requests/")
        {
            return Some(self.orders.get(id).map_or((404, String::new()), |(_, o)| {
                (200, serde_json::to_string(o).unwrap_or_default())
            }));
        }
        None
    }
}
