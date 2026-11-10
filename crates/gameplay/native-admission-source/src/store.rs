//! Typed, bounded client for Platform's `oteryn-v2-store-wire-v1` contract.
use super::{
    SourceError, TransientCapacity,
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{future::Future, pin::Pin, time::Duration};
#[path = "store_wire.rs"]
mod wire;

pub const PENDING_PATH: &str = "/internal/v1/products-entitlements/store/operations/pending";
pub const READ_PATH: &str = "/internal/v1/products-entitlements/store/operations/read";
pub const RECEIPT_PATH: &str = "/internal/v1/products-entitlements/store/operations/receipt";
pub const RECONCILE_PATH: &str = "/internal/v1/products-entitlements/store/operations/reconcile";
pub const PURCHASE_PATH: &str = "/internal/v1/products-entitlements/store/purchases";
pub const BALANCE_PATH: &str = "/internal/v1/products-entitlements/store/balance/read";
pub const HISTORY_PATH: &str = "/internal/v1/products-entitlements/store/history/read";
pub const MAX_PAGE: usize = 32;
pub const READ_REQUEST_BYTES: usize = 512;
pub const PURCHASE_REQUEST_BYTES: usize = 1024;
pub const RECEIPT_REQUEST_BYTES: usize = 1024;
pub const LIST_RESPONSE_BYTES: usize = 8192;
pub const INTENT_RESPONSE_BYTES: usize = 4096;
pub const PURCHASE_RESPONSE_BYTES: usize = 2048;
pub const BALANCE_RESPONSE_BYTES: usize = 512;
pub const DISPLAY_RESPONSE_BYTES: usize = 12288;
pub const RECEIPT_RESPONSE_BYTES: usize = 64;
pub(crate) const fn response_bytes_max(op: Operation) -> Option<usize> {
    Some(match op {
        Operation::ReadStoreBalanceV1 => BALANCE_RESPONSE_BYTES,
        Operation::ReadStoreHistoryPageV1 | Operation::ListStoreOperationStatesV1 => {
            DISPLAY_RESPONSE_BYTES
        }
        Operation::PurchaseStoreOfferV1 => PURCHASE_RESPONSE_BYTES,
        Operation::ListPendingStoreOperationsV1 => LIST_RESPONSE_BYTES,
        Operation::ReadStoreOperationIntentV1 => INTENT_RESPONSE_BYTES,
        Operation::PublishStoreDeliveryReceiptV1 => RECEIPT_RESPONSE_BYTES,
        _ => return None,
    })
}
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
    RateLimited(Duration),
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
fn typed<T: for<'de> Deserialize<'de>>(v: serde_json::Value) -> Result<T, ProtocolError> {
    serde_json::from_value(v).map_err(|_| ProtocolError::Malformed)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationKind {
    Deliver,
    Claim,
    RevokeUnclaimed,
    Reclaim,
    Flag,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingDelivery {
    pub operation_kind: OperationKind,
    pub operation_id: String,
    pub source_revision: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationCursor {
    pub after_source_revision: String,
    pub after_operation_id: String,
}
impl Default for OperationCursor {
    fn default() -> Self {
        Self {
            after_source_revision: "0".into(),
            after_operation_id: wire::NIL.into(),
        }
    }
}
impl From<&PendingDelivery> for OperationCursor {
    fn from(v: &PendingDelivery) -> Self {
        Self {
            after_source_revision: v.source_revision.clone(),
            after_operation_id: v.operation_id.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PendingPage {
    #[serde(rename = "pending")]
    pub entries: Vec<PendingDelivery>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OperationState {
    pub operation_kind: OperationKind,
    pub operation_id: String,
    pub source_revision: String,
    pub state: String,
    pub receipt_digest: Option<String>,
    pub recovery_generation: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReconcilePage {
    pub operations: Vec<OperationState>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeliveryIntent {
    pub operation_kind: OperationKind,
    pub operation_id: String,
    pub account_id: String,
    pub source_revision: String,
    pub entitlement_id: String,
    pub product_id: String,
    pub product_version: String,
    pub unit_kind: String,
    pub order_id: Option<String>,
    pub delivery_operation_id: Option<String>,
    pub catalog_revision: Option<String>,
    pub offer_key: Option<String>,
    pub grant_key: Option<String>,
    pub grant_revision: Option<String>,
    pub quantity: Option<u16>,
    pub claim_operation_id: Option<String>,
    pub claim_receipt_digest: Option<String>,
    pub reversal_operation_id: Option<String>,
    pub reversal_reason: Option<String>,
    pub shortfall_quantity: Option<u16>,
    pub issued_at_source: String,
    #[serde(skip)]
    pub binding_digest: String,
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn decode_intent(raw: &[u8]) -> Result<DeliveryIntent, ProtocolError> {
    let mut intent: DeliveryIntent = typed(wire::decode("intent", raw, INTENT_RESPONSE_BYTES)?)?;
    intent.binding_digest = digest(raw);
    Ok(intent)
}
fn ordered(entries: &[PendingDelivery], cursor: &OperationCursor) -> Result<(), ProtocolError> {
    let mut previous = (
        cursor
            .after_source_revision
            .parse::<u64>()
            .map_err(|_| ProtocolError::InvalidInput)?,
        cursor.after_operation_id.as_str(),
    );
    for e in entries {
        let key = (
            e.source_revision
                .parse::<u64>()
                .map_err(|_| ProtocolError::Malformed)?,
            e.operation_id.as_str(),
        );
        if key <= previous {
            return Err(ProtocolError::Malformed);
        }
        previous = key;
    }
    Ok(())
}
fn decode_pending(raw: &[u8]) -> Result<PendingPage, ProtocolError> {
    let page: PendingPage = typed(wire::decode("pending-response", raw, LIST_RESPONSE_BYTES)?)?;
    ordered(&page.entries, &OperationCursor::default())?;
    Ok(page)
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseStoreOffer {
    pub purchase_request_id: String,
    pub account_id: String,
    pub character_id: String,
    pub world_id: String,
    pub catalog_revision: String,
    pub offer_key: String,
    pub currency: String,
    pub unit_price: String,
    pub quantity: u8,
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
    let v = wire::decode("purchase-answer", raw, PURCHASE_RESPONSE_BYTES)?;
    Ok(match v["result"].as_str() {
        Some("ACCEPTED") => {
            let operation_ids: Vec<String> = v["units"]
                .as_array()
                .ok_or(ProtocolError::Malformed)?
                .iter()
                .map(|u| typed(u["delivery_operation_id"].clone()))
                .collect::<Result<_, _>>()?;
            let mut distinct = std::collections::BTreeSet::new();
            if operation_ids.iter().any(|id| !distinct.insert(id)) {
                return Err(ProtocolError::Malformed);
            }
            PurchaseResult::Accepted {
                order_id: typed(v["order_id"].clone())?,
                operation_ids,
            }
        }
        Some("STALE_QUOTE") => PurchaseResult::StaleQuote,
        Some("OFFER_UNAVAILABLE") => PurchaseResult::OfferUnavailable,
        Some("PURCHASE_DISABLED") => PurchaseResult::PurchaseDisabled,
        Some("INSUFFICIENT_BALANCE") => PurchaseResult::InsufficientBalance,
        Some("ACCOUNT_RESTRICTED") => PurchaseResult::AccountRestricted,
        _ => return Err(ProtocolError::UnknownResult),
    })
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Balance {
    pub currency: String,
    pub available: String,
    pub reserved: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BalancePage {
    pub account_id: String,
    pub balances: Vec<Balance>,
    pub read_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HistoryEntry {
    pub entry_id: String,
    pub kind: String,
    pub currency: String,
    pub available_delta: String,
    pub reserved_delta: String,
    pub order_id: Option<String>,
    pub offer_key: Option<String>,
    pub quantity: Option<u8>,
    pub created_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HistoryPage {
    pub account_id: String,
    pub entries: Vec<HistoryEntry>,
    pub next_before_entry_id: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportFailure {
    NotFound,
    Conflict,
    Unavailable,
    InvalidResponse,
    RateLimited(Duration),
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
            let (status, raw, retry_after) =
                http1_mtls::exchange_store(&self.descriptor, operation, body, &mut permit)
                    .await
                    .map_err(|e| match e {
                        SourceError::InvalidInput
                        | SourceError::CapacityExceeded
                        | SourceError::OversizedOk => TransportFailure::InvalidResponse,
                        _ => TransportFailure::Unavailable,
                    })?;
            classify_response(operation, status, raw, retry_after)
        })
    }
}

fn classify_response(
    operation: Operation,
    status: u16,
    raw: Vec<u8>,
    retry_after: Option<Duration>,
) -> Result<Vec<u8>, TransportFailure> {
    match status {
        200 => Ok(raw),
        404 if raw.is_empty()
            && matches!(
                operation,
                Operation::ReadStoreBalanceV1
                    | Operation::ReadStoreHistoryPageV1
                    | Operation::ReadStoreOperationIntentV1
                    | Operation::PublishStoreDeliveryReceiptV1
            ) =>
        {
            Err(TransportFailure::NotFound)
        }
        409 if raw.is_empty()
            && matches!(
                operation,
                Operation::PurchaseStoreOfferV1 | Operation::PublishStoreDeliveryReceiptV1
            ) =>
        {
            Err(TransportFailure::Conflict)
        }
        429 if raw.is_empty() => Err(retry_after
            .map(TransportFailure::RateLimited)
            .unwrap_or(TransportFailure::InvalidResponse)),
        503 if raw.is_empty() => Err(TransportFailure::Unavailable),
        _ => Err(TransportFailure::InvalidResponse),
    }
}

pub struct StoreClient {
    coins: Option<crate::coins::CoinClient>,
    transport: Box<dyn StoreTransport>,
    source_authority: String,
}
impl StoreClient {
    pub fn new(transport: Box<dyn StoreTransport>, source_authority: String) -> Self {
        Self {
            transport,
            source_authority,
            coins: None,
        }
    }
    pub fn with_coins(mut self, coins: Option<crate::coins::CoinClient>) -> Self {
        self.coins = coins;
        self
    }
    pub fn coins(&mut self) -> Option<&mut crate::coins::CoinClient> {
        self.coins.as_mut()
    }
    async fn request(&mut self, op: Operation, body: String) -> Result<Vec<u8>, ProtocolError> {
        self.transport
            .exchange(op, &body)
            .await
            .map_err(|e| match e {
                TransportFailure::NotFound => ProtocolError::NotFound,
                TransportFailure::Conflict => ProtocolError::Conflict,
                TransportFailure::Unavailable => ProtocolError::Transport,
                TransportFailure::RateLimited(delay) => ProtocolError::RateLimited(delay),
                TransportFailure::InvalidResponse => ProtocolError::Malformed,
            })
    }
    fn body(
        &self,
        shape: &str,
        mut v: serde_json::Value,
        max: usize,
    ) -> Result<String, ProtocolError> {
        v["contract_version"] = 1.into();
        v["operation"] = wire::operation(shape).into();
        v["source_authority"] = self.source_authority.clone().into();
        let body = wire::encode(shape, &v).map_err(|_| ProtocolError::InvalidInput)?;
        if body.len() > max {
            return Err(ProtocolError::Oversized);
        }
        Ok(body)
    }
    fn page_body(&self, shape: &str, cursor: &OperationCursor) -> Result<String, ProtocolError> {
        self.body(shape, serde_json::json!({"after_source_revision":cursor.after_source_revision,"after_operation_id":cursor.after_operation_id,"max_entries":MAX_PAGE}), READ_REQUEST_BYTES)
    }
    pub async fn list_pending(
        &mut self,
        cursor: &OperationCursor,
    ) -> Result<PendingPage, ProtocolError> {
        let body = self.page_body("pending-request", cursor)?;
        let page = decode_pending(
            &self
                .request(Operation::ListPendingStoreOperationsV1, body)
                .await?,
        )?;
        ordered(&page.entries, cursor)?;
        Ok(page)
    }
    pub async fn reconcile(
        &mut self,
        cursor: &OperationCursor,
    ) -> Result<ReconcilePage, ProtocolError> {
        let body = self.page_body("reconcile-request", cursor)?;
        let raw = self
            .request(Operation::ListStoreOperationStatesV1, body)
            .await?;
        let page: ReconcilePage = typed(wire::decode(
            "reconcile-response",
            &raw,
            DISPLAY_RESPONSE_BYTES,
        )?)?;
        let entries: Vec<_> = page
            .operations
            .iter()
            .map(|v| PendingDelivery {
                operation_kind: v.operation_kind,
                operation_id: v.operation_id.clone(),
                source_revision: v.source_revision.clone(),
            })
            .collect();
        ordered(&entries, cursor)?;
        Ok(page)
    }
    pub async fn read_intent(
        &mut self,
        kind: OperationKind,
        id: &str,
    ) -> Result<DeliveryIntent, ProtocolError> {
        let body = self.body("read-request", serde_json::json!({"issuer_authority":wire::ISSUER,"operation_kind":kind,"operation_id":id}), READ_REQUEST_BYTES)?;
        let intent = decode_intent(
            &self
                .request(Operation::ReadStoreOperationIntentV1, body)
                .await?,
        )?;
        if intent.operation_id != id || intent.operation_kind != kind {
            return Err(ProtocolError::Malformed);
        }
        Ok(intent)
    }
    pub async fn publish_receipt(&mut self, receipt: &[u8]) -> Result<(), ProtocolError> {
        let v = wire::decode("receipt", receipt, RECEIPT_REQUEST_BYTES)?;
        if v["source_authority"] != self.source_authority {
            return Err(ProtocolError::InvalidInput);
        }
        let body = String::from_utf8(receipt.to_vec()).map_err(|_| ProtocolError::Malformed)?;
        let raw = self
            .request(Operation::PublishStoreDeliveryReceiptV1, body)
            .await?;
        wire::decode("receipt-answer", &raw, RECEIPT_RESPONSE_BYTES)?;
        Ok(())
    }
    /// Adapt a durable Game DELIVER decision to the Platform receipt, retaining its
    /// original generation and transaction time on every replay.
    pub fn delivery_receipt(
        &self,
        intent: &DeliveryIntent,
        generation: &str,
        decided_at: &str,
    ) -> Result<Vec<u8>, ProtocolError> {
        if intent.operation_kind != OperationKind::Deliver {
            return Err(ProtocolError::InvalidInput);
        }
        let body = self.body("receipt", serde_json::json!({
            "issuer_authority":wire::ISSUER,"operation_kind":intent.operation_kind,"operation_id":intent.operation_id,
            "binding_digest":intent.binding_digest,"outcome":"COMMITTED","result_code":"STORE_OK",
            "account_id":intent.account_id,"entitlement_id":intent.entitlement_id,"product_id":intent.product_id,
            "product_version":intent.product_version,"delivered_quantity":intent.quantity,
            "recovery_generation":generation,"decided_at":decided_at,
        }), RECEIPT_REQUEST_BYTES)?;
        Ok(body.into_bytes())
    }
    /// The expected unit kind comes from the caller's admitted offer, never the answer.
    pub async fn purchase(
        &mut self,
        request: &PurchaseStoreOffer,
        expected_unit_kind: &str,
    ) -> Result<PurchaseResult, ProtocolError> {
        if !matches!(expected_unit_kind, "PREMIUM_TIME" | "COSMETIC" | "ITEM") {
            return Err(ProtocolError::InvalidInput);
        }
        let body = self
            .body(
                "purchase-request",
                serde_json::to_value(request).map_err(|_| ProtocolError::InvalidInput)?,
                PURCHASE_REQUEST_BYTES,
            )
            .map_err(|_| ProtocolError::InvalidInput)?;
        let raw = self
            .request(Operation::PurchaseStoreOfferV1, body.clone())
            .await?;
        let answer = wire::decode("purchase-answer", &raw, PURCHASE_RESPONSE_BYTES)?;
        if answer["purchase_request_id"] != request.purchase_request_id
            || answer["request_digest"] != digest(body.as_bytes())
        {
            return Err(ProtocolError::Malformed);
        }
        if answer["result"] == "ACCEPTED" {
            let units = answer["units"].as_array().ok_or(ProtocolError::Malformed)?;
            let total = request
                .unit_price
                .parse::<u64>()
                .ok()
                .and_then(|v| v.checked_mul(u64::from(request.quantity)))
                .ok_or(ProtocolError::Malformed)?;
            if answer["currency"] != request.currency
                || units.len() != usize::from(request.quantity)
                || units
                    .iter()
                    .any(|unit| unit["unit_kind"] != expected_unit_kind)
                || answer["total_price"]
                    .as_str()
                    .and_then(|v| v.parse::<u64>().ok())
                    != Some(total)
            {
                return Err(ProtocolError::Malformed);
            }
        }
        decode_purchase(&raw)
    }
    pub async fn balance(&mut self, account: &str) -> Result<BalancePage, ProtocolError> {
        let body = self.body(
            "balance-request",
            serde_json::json!({"account_id":account}),
            READ_REQUEST_BYTES,
        )?;
        let raw = self.request(Operation::ReadStoreBalanceV1, body).await?;
        let page: BalancePage = typed(wire::decode(
            "balance-response",
            &raw,
            BALANCE_RESPONSE_BYTES,
        )?)?;
        if page.account_id != account
            || (page.balances.len() == 2 && page.balances[0].currency == page.balances[1].currency)
        {
            return Err(ProtocolError::Malformed);
        }
        Ok(page)
    }
    pub async fn history(
        &mut self,
        account: &str,
        before: &str,
        max_entries: u8,
    ) -> Result<HistoryPage, ProtocolError> {
        let body = self.body("history-request", serde_json::json!({"account_id":account,"before_entry_id":before,"max_entries":max_entries}), READ_REQUEST_BYTES)?;
        let raw = self
            .request(Operation::ReadStoreHistoryPageV1, body)
            .await?;
        let page: HistoryPage = typed(wire::decode(
            "history-response",
            &raw,
            DISPLAY_RESPONSE_BYTES,
        )?)?;
        if page.account_id != account || page.entries.len() > usize::from(max_entries) {
            return Err(ProtocolError::Malformed);
        }
        let mut previous = if before == "0" {
            None
        } else {
            Some(
                before
                    .parse::<u64>()
                    .map_err(|_| ProtocolError::InvalidInput)?,
            )
        };
        for entry in &page.entries {
            let id = entry
                .entry_id
                .parse::<u64>()
                .map_err(|_| ProtocolError::Malformed)?;
            if previous.is_some_and(|p| id >= p) {
                return Err(ProtocolError::Malformed);
            }
            previous = Some(id);
        }
        if page.next_before_entry_id != "0"
            && page
                .entries
                .last()
                .is_none_or(|e| e.entry_id != page.next_before_entry_id)
        {
            return Err(ProtocolError::Malformed);
        }
        Ok(page)
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
    cursor: OperationCursor,
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
            cursor: OperationCursor::default(),
            backoff: POLL,
        }
    }
    /// Runs one bounded page. The caller owns scheduling and sleeps for the returned delay.
    pub async fn poll_once(&mut self) -> Duration {
        let page = match self.client.list_pending(&self.cursor).await {
            Ok(v) => v,
            Err(ProtocolError::RateLimited(delay)) => return delay.clamp(POLL, MAX_BACKOFF),
            Err(_) => return self.fail(),
        };
        let next = page
            .entries
            .last()
            .map(OperationCursor::from)
            .unwrap_or_default();
        for entry in page.entries {
            let intent = match self
                .client
                .read_intent(entry.operation_kind, &entry.operation_id)
                .await
            {
                Ok(v) => v,
                Err(ProtocolError::RateLimited(delay)) => return delay.clamp(POLL, MAX_BACKOFF),
                Err(_) => return self.fail(),
            };
            let receipt = match self.apply.apply(&intent).await {
                Ok(v) => v,
                Err(()) => return self.fail(),
            };
            match self.publisher.publish(&receipt).await {
                Ok(()) => {}
                Err(ProtocolError::NotFound) => self.events.unknown_operation(&entry.operation_id),
                Err(ProtocolError::RateLimited(delay)) => return delay.clamp(POLL, MAX_BACKOFF),
                Err(_) => return self.fail(),
            }
        }
        self.cursor = next;
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
