//! Bazaar pull, retained receipt replay and complete-snapshot publication.
//! The transport never infers an authoritative decision from an HTTP status.
use super::{Counter, Hash, ISSUER};
use crate::{
    SourceError, TransientCapacity,
    character_commands::{self, Decision, Input, Key, PendingEntry, Settle},
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
    runtime_status::purpose_descriptor,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{future::Future, time::Duration};

pub const PURPOSE: &str = "OTERYN_GAME_BAZAAR";
pub const RECEIPT_BYTES: usize = 270336;
pub const TRANSFERABILITY_BYTES: usize = 262144;
pub const STATUS_BYTES: usize = 2048;
pub const READ_BYTES: usize = 4096;
pub const LIST_BYTES: usize = 8192;
pub const RECONCILE_BYTES: usize = 12288;
pub const REQUEST_BYTES: usize = 512;
pub const RESPONSE_BYTES: usize = 256;
pub const PAGE: usize = 32;
const NIL: &str = "00000000-0000-0000-0000-000000000000";

pub struct BazaarDescriptor(ProducerDescriptor);
impl BazaarDescriptor {
    pub fn new(
        endpoint: (String, u16),
        peer: String,
        roots: Vec<CertificateDer<'static>>,
        chain: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
        others: &[&[CertificateDer<'static>]],
    ) -> Result<Self, SourceError> {
        purpose_descriptor(PURPOSE, endpoint, peer, roots, chain, key, others).map(Self)
    }
}

#[derive(Debug, Clone)]
pub struct ReceiptDelivery {
    pub issuer: String,
    pub operation: String,
    pub command: String,
    pub body: String,
}
#[derive(Debug, Clone)]
pub struct PublicationDelivery {
    pub subject: String,
    pub epoch: u64,
    pub revision: u64,
    pub operation: Operation,
    pub body: String,
}
#[derive(Debug, Clone)]
pub enum DeliveryKey {
    Receipt {
        issuer: String,
        operation: String,
    },
    Publication {
        subject: String,
        epoch: u64,
        revision: u64,
    },
}
#[derive(Debug, Clone, Default)]
pub struct TransportBatch {
    pub enabled: bool,
    pub receipts: Vec<ReceiptDelivery>,
    pub publications: Vec<PublicationDelivery>,
    pub watermark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteState {
    pub command: String,
    pub operation_id: String,
    pub source_revision: Counter,
    pub state: String,
    pub receipt_digest: Option<Hash>,
    pub recovery_generation: Option<Counter>,
}

pub trait Authority: Send {
    fn decide(
        &mut self,
        command: &str,
        operation: &str,
        raw: &[u8],
    ) -> impl Future<Output = Decision> + Send;
    /// Technical read evidence is separate from intent bytes. The durable
    /// owner must compare any existing decision first and prove its own refusal;
    /// an HTTP status never certifies that an earlier Game effect is absent.
    fn read_failure(
        &mut self,
        _command: &str,
        _operation: &str,
        _input: Input<'_>,
    ) -> impl Future<Output = Decision> + Send {
        async { Decision::Transient }
    }
    fn snapshot(&mut self) -> impl Future<Output = Result<TransportBatch, SourceError>> + Send;
    fn acknowledge(
        &mut self,
        key: DeliveryKey,
        digest: [u8; 32],
        status: u16,
    ) -> impl Future<Output = Result<(), SourceError>> + Send;
    fn reconcile(
        &mut self,
        state: &RemoteState,
    ) -> impl Future<Output = Result<(), SourceError>> + Send;
}

pub struct MtlsTransport {
    bazaar: BazaarDescriptor,
    receipt: ProducerDescriptor,
    capacity: TransientCapacity,
}
impl MtlsTransport {
    pub fn new(bazaar: BazaarDescriptor, receipt: ProducerDescriptor) -> Self {
        Self {
            bazaar,
            receipt,
            capacity: TransientCapacity::new(),
        }
    }
}

pub trait Exchange: Send {
    fn exchange(
        &mut self,
        operation: Operation,
        body: &str,
    ) -> impl Future<Output = Result<(u16, Vec<u8>, Option<Duration>), SourceError>> + Send;
}
impl Exchange for MtlsTransport {
    async fn exchange(
        &mut self,
        operation: Operation,
        body: &str,
    ) -> Result<(u16, Vec<u8>, Option<Duration>), SourceError> {
        let mut permit = self.capacity.try_queue()?;
        permit.try_activate()?;
        let descriptor = if operation == Operation::PublishBazaarCharacterCommandReceiptV2 {
            &self.receipt
        } else {
            &self.bazaar.0
        };
        http1_mtls::exchange_store(descriptor, operation, body, &mut permit).await
    }
}

#[derive(Serialize)]
struct List<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    after_source_revision: String,
    after_operation_id: &'a str,
    max_entries: usize,
}
pub fn encode_list(
    source: &str,
    after: Option<&Key>,
    reconcile: bool,
) -> Result<String, SourceError> {
    if !character_commands::source_authority_valid(source) {
        return Err(SourceError::InvalidInput);
    }
    let (revision, operation) = match after {
        None => (0, NIL),
        Some(key) if key.source_revision > 0 && character_commands::uuid_v7(&key.operation_id) => {
            (key.source_revision, key.operation_id.as_str())
        }
        _ => return Err(SourceError::InvalidInput),
    };
    let body = serde_json::to_string(&List {
        contract_version: 2,
        operation: if reconcile {
            "ListBazaarCommandStatesV1"
        } else {
            "ListPendingBazaarCommandsV1"
        },
        source_authority: source,
        after_source_revision: revision.to_string(),
        after_operation_id: operation,
        max_entries: PAGE,
    })
    .map_err(|_| SourceError::InvalidInput)?;
    if body.len() > REQUEST_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    Ok(body)
}
#[derive(Serialize)]
struct Read<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    issuer_authority: &'static str,
    command: &'a str,
    operation_id: &'a str,
}
pub fn encode_read(source: &str, command: &str, operation: &str) -> Result<String, SourceError> {
    if !character_commands::source_authority_valid(source)
        || !command_known(command)
        || !character_commands::uuid_v7(operation)
    {
        return Err(SourceError::InvalidInput);
    }
    let body = serde_json::to_string(&Read {
        contract_version: 2,
        operation: "ReadBazaarCommandIntentV1",
        source_authority: source,
        issuer_authority: ISSUER,
        command,
        operation_id: operation,
    })
    .map_err(|_| SourceError::InvalidInput)?;
    if body.len() > REQUEST_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    Ok(body)
}
fn command_known(command: &str) -> bool {
    matches!(
        command,
        "LockBazaarCharacter"
            | "BindBazaarCharacterTransfer"
            | "UnlockBazaarCharacter"
            | "EscrowBazaarItem"
            | "DeliverBazaarItem"
            | "ReturnBazaarItem"
            | "HoldBazaarGold"
            | "ReleaseBazaarGold"
            | "PayoutBazaarGold"
    )
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    command: String,
    operation_id: String,
    source_revision: Counter,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    contract_version: u8,
    pending: Vec<Entry>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct States {
    contract_version: u8,
    operations: Vec<RemoteState>,
}
fn exact<T: for<'d> Deserialize<'d> + Serialize>(
    raw: &[u8],
    bound: usize,
) -> Result<T, SourceError> {
    if raw.is_empty() || raw.len() > bound || raw.contains(&b'\\') {
        return Err(SourceError::InvalidInput);
    }
    let value: T = serde_json::from_slice(raw).map_err(|_| SourceError::InvalidInput)?;
    if serde_json::to_vec(&value).map_err(|_| SourceError::InvalidInput)? != raw {
        return Err(SourceError::InvalidInput);
    }
    Ok(value)
}
fn key(
    command: &str,
    operation: &str,
    revision: u64,
    previous: Option<&Key>,
) -> Result<Key, SourceError> {
    let key = Key {
        source_revision: revision,
        operation_id: operation.to_owned(),
    };
    if !command_known(command)
        || !character_commands::uuid_v7(operation)
        || previous.is_some_and(|p| p >= &key)
    {
        return Err(SourceError::InvalidInput);
    }
    Ok(key)
}
pub fn decode_pending(raw: &[u8], after: Option<&Key>) -> Result<Vec<PendingEntry>, SourceError> {
    let page: Pending = exact(raw, LIST_BYTES)?;
    if page.contract_version != 2 || page.pending.len() > PAGE {
        return Err(SourceError::InvalidInput);
    }
    let mut previous = after.cloned();
    let mut seen = std::collections::BTreeSet::new();
    let mut entries = Vec::with_capacity(page.pending.len());
    for entry in page.pending {
        let key = key(
            &entry.command,
            &entry.operation_id,
            entry.source_revision.get(),
            previous.as_ref(),
        )?;
        if !seen.insert(entry.operation_id) {
            return Err(SourceError::InvalidInput);
        }
        previous = Some(key.clone());
        entries.push(PendingEntry {
            command: entry.command,
            key,
        });
    }
    Ok(entries)
}
pub fn decode_states(raw: &[u8], after: Option<&Key>) -> Result<Vec<RemoteState>, SourceError> {
    let page: States = exact(raw, RECONCILE_BYTES)?;
    if page.contract_version != 2 || page.operations.len() > PAGE {
        return Err(SourceError::InvalidInput);
    }
    let mut previous = after.cloned();
    let mut seen = std::collections::BTreeSet::new();
    for entry in &page.operations {
        previous = Some(key(
            &entry.command,
            &entry.operation_id,
            entry.source_revision.get(),
            previous.as_ref(),
        )?);
        if !seen.insert(&entry.operation_id) {
            return Err(SourceError::InvalidInput);
        }
        let pending = matches!(entry.state.as_str(), "PENDING" | "REOPENED");
        if !(pending || matches!(entry.state.as_str(), "COMMITTED" | "REJECTED"))
            || entry.receipt_digest.is_none() != pending
            || entry.recovery_generation.is_none() != pending
        {
            return Err(SourceError::InvalidInput);
        }
    }
    Ok(page.operations)
}
pub fn accepted(raw: &[u8], publication: bool) -> bool {
    if publication {
        raw == br#"{"contract_version":2,"result":"accepted"}"#
            || raw == br#"{"contract_version":2,"result":"superseded"}"#
    } else {
        // CC §4.3 acknowledgement is unchanged by the Bazaar v2 receipt body.
        raw == br#"{"contract_version":1,"result":"accepted"}"#
    }
}

/// The HTTP parser retains Retry-After verbatim; the Bazaar consumer owns its
/// contract-specific ten-second ceiling and its own exponential backoff.
pub(crate) fn retry_delay(backoff: Duration, retry_after: Option<Duration>) -> Duration {
    retry_after
        .map_or(backoff, |delay| delay.max(backoff))
        .clamp(Duration::from_secs(1), Duration::from_secs(10))
}

/// One bounded consumer owns both cursors. Every network exchange is awaited
/// before the next starts. Its authority implementation performs all fencing.
pub struct Consumer<A, E> {
    authority: A,
    exchange: E,
    source: String,
    cursor: character_commands::Cursor,
    reconcile_cursor: character_commands::Cursor,
    last_watermark: Option<tokio::time::Instant>,
    started: tokio::time::Instant,
    retry_after: Option<Duration>,
}
impl<A: Authority, E: Exchange> Consumer<A, E> {
    pub fn new(authority: A, exchange: E, source: String) -> Self {
        Self {
            authority,
            exchange,
            source,
            cursor: character_commands::Cursor::new(Duration::ZERO),
            reconcile_cursor: character_commands::Cursor::new(Duration::ZERO),
            last_watermark: None,
            started: tokio::time::Instant::now(),
            retry_after: None,
        }
    }
    pub async fn run(mut self, event: fn(&str)) {
        let mut backoff = Duration::from_secs(1);
        loop {
            match self.cycle(event).await {
                Ok(()) => backoff = Duration::from_secs(1),
                Err(_) => backoff = (backoff * 2).min(Duration::from_secs(10)),
            }
            let delay = retry_delay(backoff, self.retry_after.take());
            tokio::time::sleep(delay).await;
        }
    }
    async fn send(
        &mut self,
        operation: Operation,
        body: &str,
    ) -> Result<(u16, Vec<u8>, Option<Duration>), SourceError> {
        let response = self.exchange.exchange(operation, body).await?;
        if matches!(response.0, 429 | 503) {
            self.retry_after = response.2;
        }
        Ok(response)
    }
    pub async fn cycle(&mut self, event: fn(&str)) -> Result<(), SourceError> {
        // Draining continues while new decisions are disabled. A flag rollback
        // cannot make custody disappear or release a hold.
        let batch = self.authority.snapshot().await?;
        for receipt in batch.receipts {
            let digest = Sha256::digest(receipt.body.as_bytes()).into();
            let key = DeliveryKey::Receipt {
                issuer: receipt.issuer,
                operation: receipt.operation,
            };
            let (status, raw, _) = self
                .send(
                    Operation::PublishBazaarCharacterCommandReceiptV2,
                    &receipt.body,
                )
                .await?;
            match status {
                200 if accepted(&raw, false) => {
                    self.authority.acknowledge(key, digest, 200).await?
                }
                400 | 401 | 404 | 409 | 413 => {
                    self.authority.acknowledge(key, digest, status).await?;
                    event("event=bazaar alarm=receipt_publication_stopped");
                }
                _ => return Err(SourceError::Unavailable),
            }
        }
        for publication in batch.publications {
            let digest = Sha256::digest(publication.body.as_bytes()).into();
            let (status, raw, _) = self.send(publication.operation, &publication.body).await?;
            if status == 200 && accepted(&raw, true) {
                self.authority
                    .acknowledge(
                        DeliveryKey::Publication {
                            subject: publication.subject,
                            epoch: publication.epoch,
                            revision: publication.revision,
                        },
                        digest,
                        200,
                    )
                    .await?;
            } else {
                event("event=bazaar alarm=projection_publication_unavailable");
                return Err(SourceError::Unavailable);
            }
        }
        // The durable authority supplies a watermark only after full resync and
        // prerequisite acknowledgements. An empty transport batch proves nothing.
        if let Some(watermark) = batch.watermark
            && self
                .last_watermark
                .is_none_or(|at| at.elapsed() >= Duration::from_secs(10))
        {
            let (status, raw, _) = self
                .send(Operation::PublishBazaarWatermarkV1, &watermark)
                .await?;
            if status != 200 || !accepted(&raw, true) {
                return Err(SourceError::Unavailable);
            }
            self.last_watermark = Some(tokio::time::Instant::now());
        }
        self.reconcile().await?;
        if !batch.enabled {
            return Ok(());
        }
        let now = self.started.elapsed();
        let after = self.cursor.next_request(now);
        let request = encode_list(&self.source, after.as_ref(), false)?;
        let (status, raw, _) = self
            .send(Operation::ListPendingBazaarCommandsV1, &request)
            .await?;
        if status != 200 {
            return Err(SourceError::Unavailable);
        }
        let entries = decode_pending(&raw, after.as_ref())?;
        self.cursor.observe(&entries);
        for entry in &entries {
            if !self.cursor.due(&entry.key, now) {
                continue;
            }
            let request = encode_read(&self.source, &entry.command, &entry.key.operation_id)?;
            let decision = match self
                .send(Operation::ReadBazaarCommandIntentV1, &request)
                .await
            {
                Ok((200, raw, _)) => {
                    self.authority
                        .decide(&entry.command, &entry.key.operation_id, &raw)
                        .await
                }
                Ok((409, raw, _)) if raw.is_empty() => {
                    event("event=bazaar alarm=intent_read_conflict");
                    self.authority
                        .read_failure(&entry.command, &entry.key.operation_id, Input::ReadConflict)
                        .await
                }
                Err(SourceError::OversizedOk) => {
                    self.authority
                        .read_failure(&entry.command, &entry.key.operation_id, Input::Oversized)
                        .await
                }
                Ok((404 | 409, _, _)) => {
                    self.cursor.settle(entry.key.clone(), Settle::Stopped, now);
                    event("event=bazaar alarm=intent_read_unavailable");
                    continue;
                }
                Ok(_) => return Err(SourceError::Unavailable),
                Err(error) => return Err(error),
            };
            let settle = match decision {
                Decision::Publish { .. } => Settle::Done,
                Decision::Conflict => {
                    event("event=bazaar alarm=operation_conflict");
                    Settle::Stopped
                }
                Decision::Transient => Settle::Retry,
            };
            self.cursor.settle(entry.key.clone(), settle, now);
        }
        self.cursor.advance(&entries);
        Ok(())
    }
    async fn reconcile(&mut self) -> Result<(), SourceError> {
        let after = self.reconcile_cursor.next_request(self.started.elapsed());
        let request = encode_list(&self.source, after.as_ref(), true)?;
        let (status, raw, _) = self
            .send(Operation::ListBazaarCommandStatesV1, &request)
            .await?;
        if status != 200 {
            return Err(SourceError::Unavailable);
        }
        let states = decode_states(&raw, after.as_ref())?;
        let mut entries = Vec::with_capacity(states.len());
        for state in &states {
            self.authority.reconcile(state).await?;
            entries.push(PendingEntry {
                command: state.command.clone(),
                key: Key {
                    source_revision: state.source_revision.get(),
                    operation_id: state.operation_id.clone(),
                },
            });
        }
        self.reconcile_cursor.advance(&entries);
        Ok(())
    }
}

pub(crate) const fn response_bytes(operation: Operation) -> Option<usize> {
    match operation {
        Operation::ListPendingBazaarCommandsV1 => Some(LIST_BYTES),
        Operation::ReadBazaarCommandIntentV1 => Some(READ_BYTES),
        Operation::ListBazaarCommandStatesV1 => Some(RECONCILE_BYTES),
        Operation::PublishBazaarTransferabilityV1
        | Operation::PublishBazaarListingStatusV1
        | Operation::PublishBazaarWatermarkV1
        | Operation::PublishBazaarCharacterCommandReceiptV2 => Some(RESPONSE_BYTES),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    const OP: &str = "0192a000-0000-7000-8000-000000000002";
    const SOURCE: &str = "oteryn:game:primary";
    #[derive(Default)]
    struct Store {
        batch: Option<TransportBatch>,
        acknowledgements: Vec<(DeliveryKey, u16)>,
        decisions: usize,
        states: usize,
        read_conflicts: usize,
        oversized: usize,
    }
    impl Authority for Store {
        async fn decide(&mut self, _: &str, _: &str, _: &[u8]) -> Decision {
            self.decisions += 1;
            Decision::Transient
        }
        async fn read_failure(&mut self, _: &str, _: &str, input: Input<'_>) -> Decision {
            let result_code = match input {
                Input::ReadConflict => {
                    self.read_conflicts += 1;
                    "CHAR_CMD_OPERATION_CONFLICT"
                }
                Input::Oversized => {
                    self.oversized += 1;
                    "CHAR_CMD_INTENT_INVALID"
                }
                _ => return Decision::Transient,
            };
            Decision::Publish {
                receipt: Vec::new(),
                result_code: result_code.to_owned(),
            }
        }
        async fn snapshot(&mut self) -> Result<TransportBatch, SourceError> {
            Ok(self.batch.take().unwrap_or_default())
        }
        async fn acknowledge(
            &mut self,
            key: DeliveryKey,
            _: [u8; 32],
            status: u16,
        ) -> Result<(), SourceError> {
            self.acknowledgements.push((key, status));
            Ok(())
        }
        async fn reconcile(&mut self, _: &RemoteState) -> Result<(), SourceError> {
            self.states += 1;
            Ok(())
        }
    }
    struct Network {
        responses: VecDeque<(Operation, u16, String)>,
        requests: Vec<(Operation, String)>,
        oversized: bool,
    }
    impl Exchange for Network {
        async fn exchange(
            &mut self,
            operation: Operation,
            body: &str,
        ) -> Result<(u16, Vec<u8>, Option<Duration>), SourceError> {
            self.requests.push((operation, body.to_owned()));
            let Some((expected, status, response)) = self.responses.pop_front() else {
                return Err(SourceError::Unavailable);
            };
            assert_eq!(operation, expected);
            if operation == Operation::ReadBazaarCommandIntentV1 && self.oversized {
                return Err(SourceError::OversizedOk);
            }
            Ok((status, response.into_bytes(), None))
        }
    }
    fn network(responses: Vec<(Operation, u16, String)>) -> Network {
        Network {
            responses: responses.into(),
            requests: Vec::new(),
            oversized: false,
        }
    }
    fn no_event(_: &str) {}
    #[test]
    fn bazaar_transport_closed_routes_and_bounds_preserve_character_v1() {
        for (operation, path, cap) in [
            (
                Operation::ListPendingBazaarCommandsV1,
                "/internal/v1/bazaar/commands/pending",
                512,
            ),
            (
                Operation::ReadBazaarCommandIntentV1,
                "/internal/v1/bazaar/commands/read",
                512,
            ),
            (
                Operation::ListBazaarCommandStatesV1,
                "/internal/v1/bazaar/commands/reconcile",
                512,
            ),
            (
                Operation::PublishBazaarTransferabilityV1,
                "/internal/v1/bazaar/transferability",
                262144,
            ),
            (
                Operation::PublishBazaarListingStatusV1,
                "/internal/v1/bazaar/status",
                2048,
            ),
            (
                Operation::PublishBazaarWatermarkV1,
                "/internal/v1/bazaar/watermark",
                512,
            ),
        ] {
            assert_eq!(operation.path(), path);
            assert_eq!(operation.request_bytes_max(), cap);
        }
        assert_eq!(
            Operation::PublishBazaarCharacterCommandReceiptV2.path(),
            Operation::PublishCharacterCommandReceiptV1.path()
        );
        assert_eq!(
            Operation::PublishCharacterCommandReceiptV1.request_bytes_max(),
            1024
        );
        assert_eq!(
            Operation::PublishBazaarCharacterCommandReceiptV2.request_bytes_max(),
            270336
        );
    }
    #[test]
    fn bazaar_transport_pages_are_exact_and_nullable_states_are_closed() -> TestResult {
        let pending = format!(
            r#"{{"contract_version":2,"pending":[{{"command":"EscrowBazaarItem","operation_id":"{OP}","source_revision":"18"}}]}}"#
        );
        let entries = decode_pending(pending.as_bytes(), None)?;
        assert_eq!(entries.len(), 1);
        assert!(decode_pending(pending.as_bytes(), Some(&entries[0].key)).is_err());
        assert!(decode_pending(format!("{pending}\n").as_bytes(), None).is_err());
        let states = format!(
            r#"{{"contract_version":2,"operations":[{{"command":"EscrowBazaarItem","operation_id":"{OP}","source_revision":"18","state":"REOPENED","receipt_digest":null,"recovery_generation":null}}]}}"#
        );
        assert_eq!(decode_states(states.as_bytes(), None)?.len(), 1);
        assert!(
            decode_states(
                states.replace("\"REOPENED\"", "\"COMMITTED\"").as_bytes(),
                None
            )
            .is_err()
        );
        assert!(
            decode_states(
                states.replace(",\"receipt_digest\":null", "").as_bytes(),
                None
            )
            .is_err()
        );
        assert!(accepted(
            br#"{"contract_version":2,"result":"superseded"}"#,
            true
        ));
        assert!(!accepted(
            br#"{"contract_version":2,"result":"superseded"}"#,
            false
        ));
        assert!(!accepted(
            br#"{"contract_version":2,"result":"accepted"}"#,
            false
        ));
        assert!(accepted(
            br#"{"contract_version":1,"result":"accepted"}"#,
            false
        ));
        Ok(())
    }
    #[test]
    fn bazaar_transport_flag_off_still_drains_and_stops_conflicting_receipt() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let store = Store {
                    batch: Some(TransportBatch {
                        enabled: false,
                        receipts: vec![ReceiptDelivery {
                            issuer: ISSUER.into(),
                            operation: OP.into(),
                            command: "EscrowBazaarItem".into(),
                            body: "retained receipt".into(),
                        }],
                        ..TransportBatch::default()
                    }),
                    ..Store::default()
                };
                let exchange = network(vec![
                    (
                        Operation::PublishBazaarCharacterCommandReceiptV2,
                        409,
                        String::new(),
                    ),
                    (
                        Operation::ListBazaarCommandStatesV1,
                        200,
                        r#"{"contract_version":2,"operations":[]}"#.into(),
                    ),
                ]);
                let mut consumer = Consumer::new(store, exchange, SOURCE.into());
                consumer.cycle(no_event).await?;
                assert_eq!(consumer.authority.decisions, 0);
                assert_eq!(consumer.authority.acknowledgements.len(), 1);
                assert_eq!(consumer.authority.acknowledgements[0].1, 409);
                assert!(consumer.exchange.responses.is_empty());
                Ok(())
            })
    }
    #[test]
    fn bazaar_transport_real_consumer_reads_exact_intent_then_calls_authority() -> TestResult {
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../../../docs/contracts/fixtures/bazaar/decisions.json"))?;
            let raw=fixture["vectors"][0]["wire"].as_str().ok_or("missing fixture")?;
            let pending=format!(r#"{{"contract_version":2,"pending":[{{"command":"EscrowBazaarItem","operation_id":"{OP}","source_revision":"18"}}]}}"#);
            let store=Store{batch:Some(TransportBatch{enabled:true,..TransportBatch::default()}),..Store::default()};
            let exchange=network(vec![(Operation::ListBazaarCommandStatesV1,200,r#"{"contract_version":2,"operations":[]}"#.into()),(Operation::ListPendingBazaarCommandsV1,200,pending),(Operation::ReadBazaarCommandIntentV1,200,raw.into())]);
            let mut consumer=Consumer::new(store,exchange,SOURCE.into());consumer.cycle(no_event).await?;
            assert_eq!(consumer.authority.decisions,1);assert!(consumer.exchange.responses.is_empty());
            assert_eq!(consumer.exchange.requests.last().map(|r|r.1.as_str()),Some(encode_read(SOURCE,"EscrowBazaarItem",OP)?.as_str()));Ok(())
        })
    }
    #[test]
    fn bazaar_consumer_submits_authenticated_read_conflict_to_durable_owner() -> TestResult {
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            let pending=format!(r#"{{"contract_version":2,"pending":[{{"command":"EscrowBazaarItem","operation_id":"{OP}","source_revision":"18"}}]}}"#);
            let store=Store{batch:Some(TransportBatch{enabled:true,..TransportBatch::default()}),..Store::default()};
            let exchange=network(vec![(Operation::ListBazaarCommandStatesV1,200,r#"{"contract_version":2,"operations":[]}"#.into()),(Operation::ListPendingBazaarCommandsV1,200,pending),(Operation::ReadBazaarCommandIntentV1,409,String::new())]);
            let mut consumer=Consumer::new(store,exchange,SOURCE.into());consumer.cycle(no_event).await?;
            assert_eq!(consumer.authority.read_conflicts,1);
            assert_eq!(consumer.authority.decisions,0);
            assert!(consumer.exchange.responses.is_empty());
            Ok(())
        })
    }
    #[test]
    fn bazaar_consumer_read404_keeps_pending_and_oversized_is_typed() -> TestResult {
        tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
            for oversized in [false,true] {
                let pending=format!(r#"{{"contract_version":2,"pending":[{{"command":"EscrowBazaarItem","operation_id":"{OP}","source_revision":"18"}}]}}"#);
                let store=Store{batch:Some(TransportBatch{enabled:true,..TransportBatch::default()}),..Store::default()};
                let mut exchange=network(vec![(Operation::ListBazaarCommandStatesV1,200,r#"{"contract_version":2,"operations":[]}"#.into()),(Operation::ListPendingBazaarCommandsV1,200,pending),(Operation::ReadBazaarCommandIntentV1,if oversized{200}else{404},String::new())]);
                exchange.oversized=oversized;
                let mut consumer=Consumer::new(store,exchange,SOURCE.into());consumer.cycle(no_event).await?;
                assert_eq!(consumer.authority.oversized,usize::from(oversized));
                assert_eq!(consumer.authority.read_conflicts,0);
                assert_eq!(consumer.authority.decisions,0);
                assert!(consumer.exchange.responses.is_empty());
            }
            Ok(())
        })
    }
}
