//! Typed pull client for the Platform-owned Store purchase and delivery contract.
//!
//! This module performs transport and strict wire decoding only. It neither owns
//! a coin balance nor creates purchase or delivery identities.

use super::{
    SourceError, TransientCapacity,
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
};
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, time::Duration};

pub const PENDING_PATH: &str = "/internal/v1/game-auth/store/deliveries/pending";
pub const READ_PATH: &str = "/internal/v1/game-auth/store/deliveries/read";
pub const RECEIPT_PATH: &str = "/internal/v1/game-auth/store/deliveries/receipt";
pub const PURCHASE_PATH: &str = "/internal/v1/game-auth/store/purchase";
pub const BALANCE_PATH: &str = "/internal/v1/game-auth/store/balance";
pub const HISTORY_PATH: &str = "/internal/v1/game-auth/store/history";
pub const MAX_PAGE: usize = 32;
pub const READ_REQUEST_BYTES: usize = 512;
pub const PURCHASE_REQUEST_BYTES: usize = 2048;
pub const RECEIPT_REQUEST_BYTES: usize = 2048;
pub const LIST_RESPONSE_BYTES: usize = 8192;
pub const INTENT_RESPONSE_BYTES: usize = 8192;
pub const PURCHASE_RESPONSE_BYTES: usize = 2048;
pub const DISPLAY_RESPONSE_BYTES: usize = 16384;
pub const RECEIPT_RESPONSE_BYTES: usize = 256;
pub const POLL: Duration = Duration::from_secs(1);
pub const MAX_BACKOFF: Duration = Duration::from_secs(30);

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidInput,
    Malformed,
    UnknownResult,
    Oversized,
    Conflict,
    NotFound,
    Transport,
}

fn plain(raw: &[u8], max: usize) -> Result<(), ProtocolError> {
    if raw.len() > max {
        return Err(ProtocolError::Oversized);
    }
    if raw.is_empty() || raw.iter().any(|b| *b < 0x20 || *b == b'\\') {
        return Err(ProtocolError::Malformed);
    }
    Ok(())
}
fn token(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
fn uuid7(value: &str) -> bool {
    super::runtime_status::canonical_uuid(value, Some(b'7'))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingDelivery {
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryIntent {
    pub operation_id: String,
    pub account_id: String,
    pub character_id: Option<String>,
    pub world_id: Option<String>,
    pub fulfilment_unit_id: String,
    pub catalog_revision: String,
    pub offer_key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Page<T> {
    contract_version: u8,
    entries: Vec<T>,
    next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPage {
    pub entries: Vec<PendingDelivery>,
    pub next_cursor: Option<String>,
}

fn decode_page<T: for<'de> Deserialize<'de>>(
    raw: &[u8],
    max: usize,
) -> Result<Page<T>, ProtocolError> {
    plain(raw, max)?;
    let page: Page<T> = serde_json::from_slice(raw).map_err(|_| ProtocolError::Malformed)?;
    if page.contract_version != 1
        || page.entries.len() > MAX_PAGE
        || page.next_cursor.as_ref().is_some_and(|v| !token(v, 256))
    {
        return Err(ProtocolError::Malformed);
    }
    Ok(page)
}

fn decode_pending(raw: &[u8]) -> Result<PendingPage, ProtocolError> {
    let page = decode_page::<PendingDelivery>(raw, LIST_RESPONSE_BYTES)?;
    if page.entries.iter().any(|v| !uuid7(&v.operation_id)) {
        return Err(ProtocolError::Malformed);
    }
    Ok(PendingPage {
        entries: page.entries,
        next_cursor: page.next_cursor,
    })
}

fn decode_intent(raw: &[u8]) -> Result<DeliveryIntent, ProtocolError> {
    plain(raw, INTENT_RESPONSE_BYTES)?;
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Wire {
        contract_version: u8,
        intent: DeliveryIntent,
    }
    let wire: Wire = serde_json::from_slice(raw).map_err(|_| ProtocolError::Malformed)?;
    let v = wire.intent;
    if wire.contract_version != 1
        || !uuid7(&v.operation_id)
        || !token(&v.account_id, 128)
        || v.character_id.as_ref().is_some_and(|x| !token(x, 128))
        || v.world_id.as_ref().is_some_and(|x| !token(x, 128))
        || !token(&v.fulfilment_unit_id, 128)
        || !token(&v.catalog_revision, 128)
        || !token(&v.offer_key, 128)
    {
        return Err(ProtocolError::Malformed);
    }
    Ok(v)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PurchaseStoreOffer {
    pub purchase_request_id: String,
    pub account_id: String,
    pub character_id: String,
    pub offer_key: String,
    pub catalog_revision: String,
    pub price: u64,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseResult {
    Accepted {
        order_id: String,
        operation_ids: Vec<String>,
    },
    StaleQuote,
    OfferUnavailable,
    PurchaseDisabled,
    InsufficientBalance,
    AccountRestricted,
}

fn decode_purchase(raw: &[u8]) -> Result<PurchaseResult, ProtocolError> {
    plain(raw, PURCHASE_RESPONSE_BYTES)?;
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Wire {
        contract_version: u8,
        result: String,
        order_id: Option<String>,
        operation_ids: Option<Vec<String>>,
    }
    let w: Wire = serde_json::from_slice(raw).map_err(|_| ProtocolError::Malformed)?;
    if w.contract_version != 1 {
        return Err(ProtocolError::Malformed);
    }
    match w.result.as_str() {
        "ACCEPTED" => {
            let (Some(order), Some(ids)) = (w.order_id, w.operation_ids) else {
                return Err(ProtocolError::Malformed);
            };
            if !uuid7(&order)
                || ids.is_empty()
                || ids.len() > MAX_PAGE
                || ids.iter().any(|v| !uuid7(v))
            {
                return Err(ProtocolError::Malformed);
            }
            Ok(PurchaseResult::Accepted {
                order_id: order,
                operation_ids: ids,
            })
        }
        _ if w.order_id.is_some() || w.operation_ids.is_some() => Err(ProtocolError::Malformed),
        "STALE_QUOTE" => Ok(PurchaseResult::StaleQuote),
        "OFFER_UNAVAILABLE" => Ok(PurchaseResult::OfferUnavailable),
        "PURCHASE_DISABLED" => Ok(PurchaseResult::PurchaseDisabled),
        "INSUFFICIENT_BALANCE" => Ok(PurchaseResult::InsufficientBalance),
        "ACCOUNT_RESTRICTED" => Ok(PurchaseResult::AccountRestricted),
        _ => Err(ProtocolError::UnknownResult),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Balance {
    pub currency: String,
    pub balance: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryEntry {
    pub order_id: String,
    pub offer_key: String,
    pub price: u64,
    pub currency: String,
    pub purchased_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportFailure {
    NotFound,
    Conflict,
    Unavailable,
    InvalidResponse,
}
pub trait StoreTransport: Send {
    fn exchange<'a>(
        &'a mut self,
        operation: Operation,
        body: &'a str,
    ) -> BoxFuture<'a, Result<Vec<u8>, TransportFailure>>;
}

pub struct MtlsStoreTransport {
    descriptor: ProducerDescriptor,
    capacity: TransientCapacity,
}
impl MtlsStoreTransport {
    pub const fn new(descriptor: ProducerDescriptor) -> Self {
        Self {
            descriptor,
            capacity: TransientCapacity::new(),
        }
    }
}
impl StoreTransport for MtlsStoreTransport {
    fn exchange<'a>(
        &'a mut self,
        operation: Operation,
        body: &'a str,
    ) -> BoxFuture<'a, Result<Vec<u8>, TransportFailure>> {
        Box::pin(async move {
            let mut permit = self
                .capacity
                .try_queue()
                .map_err(|_| TransportFailure::Unavailable)?;
            permit
                .try_activate()
                .map_err(|_| TransportFailure::Unavailable)?;
            let (status, raw) =
                http1_mtls::exchange_with_status(&self.descriptor, operation, body, &mut permit)
                    .await
                    .map_err(|_| TransportFailure::Unavailable)?;
            match status {
                200 => Ok(raw),
                404 if raw.is_empty() => Err(TransportFailure::NotFound),
                409 if raw.is_empty() => Err(TransportFailure::Conflict),
                429 | 503 if raw.is_empty() => Err(TransportFailure::Unavailable),
                _ => Err(TransportFailure::InvalidResponse),
            }
        })
    }
}

pub struct StoreClient {
    transport: Box<dyn StoreTransport>,
}
impl StoreClient {
    pub fn new(transport: Box<dyn StoreTransport>) -> Self {
        Self { transport }
    }
    async fn request(&mut self, op: Operation, body: String) -> Result<Vec<u8>, ProtocolError> {
        self.transport
            .exchange(op, &body)
            .await
            .map_err(|e| match e {
                TransportFailure::NotFound => ProtocolError::NotFound,
                TransportFailure::Conflict => ProtocolError::Conflict,
                TransportFailure::Unavailable => ProtocolError::Transport,
                TransportFailure::InvalidResponse => ProtocolError::Malformed,
            })
    }
    pub async fn list_pending(
        &mut self,
        cursor: Option<&str>,
    ) -> Result<PendingPage, ProtocolError> {
        if cursor.is_some_and(|v| !token(v, 256)) {
            return Err(ProtocolError::InvalidInput);
        }
        let body = serde_json::json!({"contract_version":1,"cursor":cursor,"max_entries":MAX_PAGE})
            .to_string();
        decode_pending(
            &self
                .request(Operation::ListPendingStoreDeliveriesV1, body)
                .await?,
        )
    }
    pub async fn read_intent(&mut self, id: &str) -> Result<DeliveryIntent, ProtocolError> {
        if !uuid7(id) {
            return Err(ProtocolError::InvalidInput);
        }
        let body = serde_json::json!({"contract_version":1,"operation_id":id}).to_string();
        let intent = decode_intent(
            &self
                .request(Operation::ReadStoreDeliveryIntentV1, body)
                .await?,
        )?;
        if intent.operation_id != id {
            return Err(ProtocolError::Malformed);
        }
        Ok(intent)
    }
    pub async fn publish_receipt(&mut self, receipt: &[u8]) -> Result<(), ProtocolError> {
        plain(receipt, RECEIPT_REQUEST_BYTES)?;
        let body = std::str::from_utf8(receipt)
            .map_err(|_| ProtocolError::Malformed)?
            .to_owned();
        let raw = self.request(Operation::PublishStoreReceiptV1, body).await?;
        plain(&raw, RECEIPT_RESPONSE_BYTES)?;
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Accepted {
            contract_version: u8,
            result: String,
        }
        let v: Accepted = serde_json::from_slice(&raw).map_err(|_| ProtocolError::Malformed)?;
        if v.contract_version == 1 && v.result == "accepted" {
            Ok(())
        } else {
            Err(ProtocolError::UnknownResult)
        }
    }
    pub async fn purchase(
        &mut self,
        request: &PurchaseStoreOffer,
    ) -> Result<PurchaseResult, ProtocolError> {
        if !uuid7(&request.purchase_request_id)
            || !token(&request.account_id, 128)
            || !token(&request.character_id, 128)
            || !token(&request.offer_key, 128)
            || !token(&request.catalog_revision, 128)
            || !token(&request.currency, 32)
        {
            return Err(ProtocolError::InvalidInput);
        }
        #[derive(Serialize)]
        struct PurchaseWire<'a> {
            contract_version: u8,
            operation: &'static str,
            #[serde(flatten)]
            request: &'a PurchaseStoreOffer,
        }
        let body = serde_json::to_string(&PurchaseWire {
            contract_version: 1,
            operation: "PurchaseStoreOffer",
            request,
        })
        .map_err(|_| ProtocolError::InvalidInput)?;
        if body.len() > PURCHASE_REQUEST_BYTES {
            return Err(ProtocolError::Oversized);
        }
        decode_purchase(&self.request(Operation::PurchaseStoreOfferV1, body).await?)
    }
    pub async fn balance(
        &mut self,
        account: &str,
        currency: &str,
    ) -> Result<Balance, ProtocolError> {
        if !token(account, 128) || !token(currency, 32) {
            return Err(ProtocolError::InvalidInput);
        }
        let body =
            serde_json::json!({"contract_version":1,"account_id":account,"currency":currency})
                .to_string();
        let raw = self.request(Operation::ReadStoreBalanceV1, body).await?;
        plain(&raw, DISPLAY_RESPONSE_BYTES)?;
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct W {
            contract_version: u8,
            balance: Balance,
        }
        let w: W = serde_json::from_slice(&raw).map_err(|_| ProtocolError::Malformed)?;
        if w.contract_version != 1 || !token(&w.balance.currency, 32) {
            return Err(ProtocolError::Malformed);
        }
        Ok(w.balance)
    }
    pub async fn history(
        &mut self,
        account: &str,
        cursor: Option<&str>,
    ) -> Result<HistoryPage, ProtocolError> {
        if !token(account, 128) || cursor.is_some_and(|v| !token(v, 256)) {
            return Err(ProtocolError::InvalidInput);
        }
        let body=serde_json::json!({"contract_version":1,"account_id":account,"cursor":cursor,"max_entries":MAX_PAGE}).to_string();
        let p = decode_page::<HistoryEntry>(
            &self.request(Operation::ReadStoreHistoryV1, body).await?,
            DISPLAY_RESPONSE_BYTES,
        )?;
        if p.entries.iter().any(|v| {
            !uuid7(&v.order_id)
                || !token(&v.offer_key, 128)
                || !token(&v.currency, 32)
                || v.purchased_at.len() > 64
        }) {
            return Err(ProtocolError::Malformed);
        }
        Ok(HistoryPage {
            entries: p.entries,
            next_cursor: p.next_cursor,
        })
    }
}

pub trait ApplyDelivery: Send {
    fn apply<'a>(&'a mut self, intent: &'a DeliveryIntent) -> BoxFuture<'a, Result<Vec<u8>, ()>>;
}
pub trait PublishReceipt: Send {
    fn publish<'a>(&'a mut self, receipt: &'a [u8]) -> BoxFuture<'a, Result<(), ProtocolError>>;
}
impl PublishReceipt for StoreClient {
    fn publish<'a>(&'a mut self, receipt: &'a [u8]) -> BoxFuture<'a, Result<(), ProtocolError>> {
        Box::pin(async move { self.publish_receipt(receipt).await })
    }
}
pub trait ReceiptEvents: Send {
    fn unknown_operation(&mut self, operation_id: &str);
}

pub struct StorePoller {
    client: StoreClient,
    apply: Box<dyn ApplyDelivery>,
    publisher: Box<dyn PublishReceipt>,
    events: Box<dyn ReceiptEvents>,
    cursor: Option<String>,
    backoff: Duration,
}
impl StorePoller {
    pub fn new(
        client: StoreClient,
        apply: Box<dyn ApplyDelivery>,
        publisher: Box<dyn PublishReceipt>,
        events: Box<dyn ReceiptEvents>,
    ) -> Self {
        Self {
            client,
            apply,
            publisher,
            events,
            cursor: None,
            backoff: POLL,
        }
    }
    /// Runs one bounded page. The caller owns scheduling and sleeps for the returned delay.
    pub async fn poll_once(&mut self) -> Duration {
        let page = match self.client.list_pending(self.cursor.as_deref()).await {
            Ok(v) => v,
            Err(_) => return self.fail(),
        };
        for entry in page.entries {
            let intent = match self.client.read_intent(&entry.operation_id).await {
                Ok(v) => v,
                Err(_) => return self.fail(),
            };
            let receipt = match self.apply.apply(&intent).await {
                Ok(v) => v,
                Err(()) => return self.fail(),
            };
            match self.publisher.publish(&receipt).await {
                Ok(()) => {}
                Err(ProtocolError::NotFound) => self.events.unknown_operation(&entry.operation_id),
                Err(_) => return self.fail(),
            }
        }
        self.cursor = page.next_cursor;
        self.backoff = POLL;
        POLL
    }
    fn fail(&mut self) -> Duration {
        let wait = self.backoff;
        self.backoff = self.backoff.saturating_mul(2).min(MAX_BACKOFF);
        wait
    }
}

impl From<SourceError> for ProtocolError {
    fn from(_: SourceError) -> Self {
        Self::Transport
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
