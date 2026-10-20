//! Character command consumer transport (`oteryn-game-character-authority-commands-v1`
//! §3, §4, §8).
//!
//! Game lists the PENDING Platform command intents, reads each one over its own
//! purpose-bound mTLS identity, decides it in the Character Authority and
//! publishes the stored receipt. The consumer runs only when
//! `[platform.character_commands]` is configured, and it is not configured by
//! default (§14 step 4). Logs carry the wire operation, the command, the
//! result code and timings only, never an `AccountId`, a name or a list.

use super::{
    SourceError, TransientCapacity,
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
    runtime_status::{ReportClock, canonical_uuid, purpose_descriptor},
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    time::Duration,
};

/// Compiled namespace of the command descriptor; never a configured source
/// authority, so no other purpose's descriptor matches it.
pub const PURPOSE: &str = "OTERYN_GAME_CHARACTER_COMMANDS";
/// The only issuer Game accepts intents from (§3), sent on every read.
pub const ISSUER_AUTHORITY: &str = "OTERYN_PLATFORM_CHARACTER_AUTHORITY";
/// The one command this consumer decides; any other is `CHAR_CMD_UNSUPPORTED`.
pub const CREATE_COMMAND: &str = "CreateCharacter";
pub const PENDING_PATH: &str = "/internal/v1/game-auth/character-commands/pending";
pub const READ_PATH: &str = "/internal/v1/game-auth/character-commands/read";
pub const RECEIPT_PATH: &str = "/internal/v1/game-auth/character-commands/receipt";
/// `CHARCMD-PENDING-PAGE`.
pub const MAX_ENTRIES: usize = 32;
/// `CHARCMD-LIST-REQUEST-BYTES`.
pub const LIST_REQUEST_BYTES: usize = 512;
/// `CHARCMD-READ-REQUEST-BYTES`.
pub const READ_REQUEST_BYTES: usize = 512;
/// `CHARCMD-RECEIPT-REQUEST-BYTES`.
pub const RECEIPT_REQUEST_BYTES: usize = 1024;
/// `CHARCMD-LIST-RESPONSE-BYTES`.
pub const LIST_RESPONSE_BYTES: usize = 8192;
/// `CHARCMD-INTENT-BYTES`.
pub const INTENT_RESPONSE_BYTES: usize = 4096;
/// `CHARCMD-RECEIPT-RESPONSE-BYTES`.
pub const RECEIPT_RESPONSE_BYTES: usize = 256;
/// List cadence while enabled (§3).
pub const POLL: Duration = Duration::from_secs(1);
/// Longest backoff on `429` or `503` (§3).
pub const MAX_BACKOFF: Duration = Duration::from_secs(10);
/// A rescan starts after this many consecutive full pages (§4.1) ...
pub const RESCAN_FULL_PAGES: u32 = 8;
/// ... or this long after the previous one, whichever comes first.
pub const RESCAN_PERIOD: Duration = Duration::from_secs(30);
/// Operator alarm thresholds of one rescan (§4.1).
pub const RESCAN_LISTED_ALARM: usize = 192;
pub const RESCAN_UNSEEN_ALARM: usize = 16;
/// Bound of the local known-key set: twice `CHARCMD-PENDING-BACKLOG` (256).
pub const KNOWN_KEYS: usize = 512;
/// A settled key is listed again only after this long, so an intent that a
/// restore reopened (§5.4) is republished without a busy loop.
pub const SETTLED_RECHECK: Duration = Duration::from_secs(60);

/// The response bound of a character-command operation, `None` for others.
pub(super) const fn response_bytes(operation: Operation) -> Option<usize> {
    match operation {
        Operation::ListPendingCharacterCommandsV1 => Some(LIST_RESPONSE_BYTES),
        Operation::ReadCharacterCommandIntentV1 => Some(INTENT_RESPONSE_BYTES),
        Operation::PublishCharacterCommandReceiptV1 => Some(RECEIPT_RESPONSE_BYTES),
        _ => None,
    }
}

/// The §4.1 continuation key: `source_revision` as uint64, then `operation_id`
/// as bytes (canonical lowercase hex orders as its bytes).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    pub source_revision: u64,
    pub operation_id: String,
}

/// One PENDING entry of a list response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingEntry {
    pub command: String,
    pub key: Key,
}

#[must_use]
pub fn source_authority_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}

/// A `command` string: 1..64 bytes of ASCII letters, digits and `_` (§4.3).
#[must_use]
pub fn command_valid(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// A canonical non-zero decimal uint64 string (§4).
#[must_use]
pub fn counter(value: &str) -> Option<u64> {
    let b = value.as_bytes();
    if b.is_empty() || b.len() > 20 || b[0] == b'0' || !b.iter().all(u8::is_ascii_digit) {
        return None;
    }
    value.parse().ok()
}

/// A canonical lowercase non-nil UUIDv7 string (§4).
#[must_use]
pub fn uuid_v7(value: &str) -> bool {
    canonical_uuid(value, Some(b'7'))
}

/// The §4 body rules every decoder applies before parsing: raw UTF-8 without
/// string escapes, control bytes or insignificant whitespace (no wire value
/// holds a space).
fn plain(raw: &[u8]) -> bool {
    raw.iter().all(|&b| b > 0x20 && b != b'\\')
}

#[derive(Serialize)]
struct ListWire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    after_source_revision: String,
    after_operation_id: &'a str,
    max_entries: usize,
}

const NIL_UUID: &str = "00000000-0000-0000-0000-000000000000";

/// Exact §4.1 request; `None` starts at `("0", nil)`.
pub fn encode_list(source_authority: &str, after: Option<&Key>) -> Result<String, SourceError> {
    if !source_authority_valid(source_authority) {
        return Err(SourceError::InvalidInput);
    }
    let (revision, operation) = match after {
        None => (0, NIL_UUID),
        Some(key) if key.source_revision != 0 && uuid_v7(&key.operation_id) => {
            (key.source_revision, key.operation_id.as_str())
        }
        Some(_) => return Err(SourceError::InvalidInput),
    };
    let body = serde_json::to_string(&ListWire {
        contract_version: 1,
        operation: "ListPendingCharacterCommandsV1",
        source_authority,
        after_source_revision: revision.to_string(),
        after_operation_id: operation,
        max_entries: MAX_ENTRIES,
    })
    .map_err(|_| SourceError::InvalidInput)?;
    if body.len() > LIST_REQUEST_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    Ok(body)
}

#[derive(Serialize)]
struct ReadWire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    issuer_authority: &'static str,
    command: &'a str,
    operation_id: &'a str,
}

/// Exact §4.2 request.
pub fn encode_read(
    source_authority: &str,
    command: &str,
    operation_id: &str,
) -> Result<String, SourceError> {
    if !source_authority_valid(source_authority)
        || !command_valid(command)
        || !uuid_v7(operation_id)
    {
        return Err(SourceError::InvalidInput);
    }
    let body = serde_json::to_string(&ReadWire {
        contract_version: 1,
        operation: "ReadCharacterCommandIntentV1",
        source_authority,
        issuer_authority: ISSUER_AUTHORITY,
        command,
        operation_id,
    })
    .map_err(|_| SourceError::InvalidInput)?;
    if body.len() > READ_REQUEST_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    Ok(body)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingWire {
    contract_version: u8,
    pending: Vec<EntryWire>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryWire {
    command: String,
    operation_id: String,
    source_revision: String,
}

/// Exact §4.1 response: at most 32 entries strictly above `after`, ascending
/// by key, with unique `operation_id`s. Anything else is malformed.
pub fn decode_pending(raw: &[u8], after: Option<&Key>) -> Result<Vec<PendingEntry>, SourceError> {
    if raw.len() > LIST_RESPONSE_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    if !plain(raw) {
        return Err(SourceError::InvalidInput);
    }
    let wire: PendingWire = serde_json::from_slice(raw).map_err(|_| SourceError::InvalidInput)?;
    if wire.contract_version != 1 || wire.pending.len() > MAX_ENTRIES {
        return Err(SourceError::InvalidInput);
    }
    let mut operations = BTreeSet::new();
    let mut previous = after.cloned();
    let mut entries = Vec::with_capacity(wire.pending.len());
    for entry in wire.pending {
        let source_revision = counter(&entry.source_revision).ok_or(SourceError::InvalidInput)?;
        if !command_valid(&entry.command) || !uuid_v7(&entry.operation_id) {
            return Err(SourceError::InvalidInput);
        }
        let key = Key {
            source_revision,
            operation_id: entry.operation_id,
        };
        if previous.as_ref().is_some_and(|previous| key <= *previous)
            || !operations.insert(key.operation_id.clone())
        {
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptedWire {
    contract_version: u8,
    result: String,
}

/// Exact §4.3 success body `{"contract_version":1,"result":"accepted"}`.
pub fn decode_receipt_response(raw: &[u8]) -> Result<(), SourceError> {
    if raw.len() > RECEIPT_RESPONSE_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    if !plain(raw) {
        return Err(SourceError::InvalidInput);
    }
    match serde_json::from_slice::<AcceptedWire>(raw) {
        Ok(AcceptedWire {
            contract_version: 1,
            result,
        }) if result == "accepted" => Ok(()),
        _ => Err(SourceError::InvalidInput),
    }
}

/// Why an exchange gave no `200` body. Failures are empty bodies (§4.3); a
/// failure status with a body is an invalid response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// `400`.
    Malformed,
    /// `401`.
    Unauthenticated,
    /// `404`.
    NotFound,
    /// `409`.
    Conflict,
    /// `429`.
    RateLimited,
    /// A response outside the contract.
    InvalidResponse,
    /// A `200` body over the operation's bound.
    Oversized,
    /// `503`, any other status or transport failure.
    Unavailable,
}

impl Failure {
    /// `429` and `503` back the consumer off (§3).
    #[must_use]
    pub const fn backs_off(self) -> bool {
        matches!(self, Self::RateLimited | Self::Unavailable)
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Malformed => "malformed",
            Self::Unauthenticated => "unauthenticated",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::RateLimited => "rate_limited",
            Self::InvalidResponse => "invalid_response",
            Self::Oversized => "oversized",
            Self::Unavailable => "unavailable",
        }
    }
}

#[must_use]
pub fn classify(status: u16, raw: &[u8]) -> Failure {
    let class = match status {
        400 => Failure::Malformed,
        401 => Failure::Unauthenticated,
        404 => Failure::NotFound,
        409 => Failure::Conflict,
        429 => Failure::RateLimited,
        _ => return Failure::Unavailable,
    };
    if raw.is_empty() {
        class
    } else {
        Failure::InvalidResponse
    }
}

/// Character-command client identity (§3): its own certificate and key, never
/// those of the evidence, runtime-status or projection purposes.
pub struct CommandDescriptor(ProducerDescriptor);

impl CommandDescriptor {
    /// `other_purposes` are the certificate chains of every other Platform
    /// identity this host holds; none may share the command public key.
    pub fn new(
        connect_endpoint: (String, u16),
        peer_name: String,
        roots: Vec<CertificateDer<'static>>,
        client_chain: Vec<CertificateDer<'static>>,
        client_key: PrivateKeyDer<'static>,
        other_purposes: &[&[CertificateDer<'static>]],
    ) -> Result<Self, SourceError> {
        purpose_descriptor(
            PURPOSE,
            connect_endpoint,
            peer_name,
            roots,
            client_chain,
            client_key,
            other_purposes,
        )
        .map(Self)
    }
}

/// Where exchanges go: Platform over mTLS, or a test source.
pub trait CommandSource: Send {
    /// The `200` body, or why there is none.
    fn exchange(
        &mut self,
        operation: Operation,
        body: &str,
    ) -> impl Future<Output = Result<Vec<u8>, Failure>> + Send;
}

/// The production source: one exchange at a time on its own capacity.
pub struct MtlsSource {
    descriptor: CommandDescriptor,
    capacity: TransientCapacity,
}

impl MtlsSource {
    #[must_use]
    pub const fn new(descriptor: CommandDescriptor) -> Self {
        Self {
            descriptor,
            capacity: TransientCapacity::new(),
        }
    }
}

impl CommandSource for MtlsSource {
    async fn exchange(&mut self, operation: Operation, body: &str) -> Result<Vec<u8>, Failure> {
        let mut permit = self
            .capacity
            .try_queue()
            .map_err(|_| Failure::Unavailable)?;
        permit.try_activate().map_err(|_| Failure::Unavailable)?;
        let (status, raw) =
            http1_mtls::exchange_with_status(&self.descriptor.0, operation, body, &mut permit)
                .await
                .map_err(|error| match error {
                    SourceError::OversizedOk => Failure::Oversized,
                    _ => Failure::Unavailable,
                })?;
        if status == 200 {
            Ok(raw)
        } else {
            Err(classify(status, &raw))
        }
    }
}

/// What the Character Authority decides from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input<'a> {
    /// The exact intent bytes of a `200` read.
    Intent(&'a [u8]),
    /// The read answered `409`: Platform holds this operation for another command.
    ReadConflict,
    /// A command this consumer does not decide; it is not read.
    Unsupported,
    /// The read answered `200` with a body over the intent bound (§3): not the
    /// §4.2 wire, so the intent is invalid.
    Oversized,
}

/// The Character Authority's answer for one operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Publish these stored receipt bytes.
    Publish {
        receipt: Vec<u8>,
        result_code: String,
    },
    /// A stored receipt holds this operation with another binding: nothing is
    /// written or published, and the operator is alarmed (§5.2).
    Conflict,
    /// A dependency was unavailable: nothing was stored; retry later (§6).
    Transient,
}

/// The Character Authority side of the consumer.
pub trait Decider: Send {
    fn decide(
        &mut self,
        command: &str,
        operation_id: &str,
        input: Input<'_>,
    ) -> impl Future<Output = Decision> + Send;
}

#[derive(Debug, Clone, Copy)]
struct Known {
    not_before: Duration,
    attempt: u32,
    stopped: bool,
}

#[derive(Debug, Clone)]
struct Rescan {
    target: Key,
    listed: usize,
    unseen: usize,
    alarmed: bool,
}

/// How one key ended for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settle {
    /// Decided and published, or stopped with an alarm.
    Done,
    /// Receipt refused (`404`/`409`): not republished while it stays listed (§4.3).
    Stopped,
    /// Transient: retry on the key's own backoff.
    Retry,
}

/// The §4.1 paging and rescan state, with the known-key set that keeps a
/// rescan list-only for keys already decided or on their own backoff.
#[derive(Debug, Clone, Default)]
pub struct Cursor {
    after: Option<Key>,
    full_pages: u32,
    last_rescan: Duration,
    rescan: Option<Rescan>,
    known: BTreeMap<Key, Known>,
    /// Keys listed since the last pass started at `("0", nil)`; `None` once it
    /// overflowed, which only skips pruning for that pass.
    listed: Option<BTreeSet<Key>>,
}

impl Cursor {
    #[must_use]
    pub fn new(now: Duration) -> Self {
        Self {
            last_rescan: now,
            listed: Some(BTreeSet::new()),
            ..Self::default()
        }
    }

    /// The continuation key of the next list request, starting a rescan when
    /// one is due.
    pub fn next_request(&mut self, now: Duration) -> Option<Key> {
        if self.rescan.is_none() {
            match &self.after {
                // A pass from the start lists every key: it is a rescan.
                None => {
                    self.last_rescan = now;
                    self.full_pages = 0;
                    self.listed = Some(BTreeSet::new());
                }
                Some(reached)
                    if self.full_pages >= RESCAN_FULL_PAGES
                        || now.saturating_sub(self.last_rescan) >= RESCAN_PERIOD =>
                {
                    self.rescan = Some(Rescan {
                        target: reached.clone(),
                        listed: 0,
                        unseen: 0,
                        alarmed: false,
                    });
                    self.after = None;
                    self.full_pages = 0;
                    self.last_rescan = now;
                }
                Some(_) => {}
            }
        }
        self.after.clone()
    }

    #[must_use]
    pub fn rescanning(&self) -> bool {
        self.rescan.is_some()
    }

    /// Count a listed page against the rescan alarm thresholds, before any of
    /// its keys is processed. True once when a threshold is first exceeded.
    pub fn observe(&mut self, entries: &[PendingEntry]) -> bool {
        let Some(rescan) = &mut self.rescan else {
            return false;
        };
        rescan.listed = rescan.listed.saturating_add(entries.len());
        rescan.unseen = rescan.unseen.saturating_add(
            entries
                .iter()
                .filter(|e| e.key <= rescan.target && !self.known.contains_key(&e.key))
                .count(),
        );
        let exceeded = rescan.listed > RESCAN_LISTED_ALARM || rescan.unseen > RESCAN_UNSEEN_ALARM;
        let alarm = exceeded && !rescan.alarmed;
        rescan.alarmed |= exceeded;
        alarm
    }

    /// Whether a listed key is processed now: unknown, or its backoff passed.
    #[must_use]
    pub fn due(&self, key: &Key, now: Duration) -> bool {
        self.known.get(key).is_none_or(|k| now >= k.not_before)
    }

    /// Record how a processed key ended.
    pub fn settle(&mut self, key: Key, settle: Settle, now: Duration) {
        let attempt = match settle {
            Settle::Done | Settle::Stopped => 0,
            Settle::Retry => self.known.get(&key).map_or(0, |k| k.attempt) + 1,
        };
        let delay = match settle {
            Settle::Done => SETTLED_RECHECK,
            Settle::Stopped => Duration::MAX,
            Settle::Retry => POLL.saturating_mul(1 << attempt.min(4)).min(MAX_BACKOFF),
        };
        if !self.known.contains_key(&key) && self.known.len() >= KNOWN_KEYS {
            // A stopped key must stay suppressed until a pass omits it, so the
            // bound evicts any other key first.
            let evict = self
                .known
                .iter()
                .find(|(_, k)| !k.stopped)
                .or_else(|| self.known.first_key_value())
                .map(|(k, _)| k.clone());
            if let Some(evict) = evict {
                self.known.remove(&evict);
            }
        }
        self.known.insert(
            key,
            Known {
                not_before: now.saturating_add(delay),
                attempt,
                stopped: settle == Settle::Stopped,
            },
        );
    }

    /// Advance past a page whose keys were all processed or skipped. A short
    /// page ends the pass: the next request starts at `("0", nil)` and keys
    /// no longer listed leave the known set.
    pub fn advance(&mut self, entries: &[PendingEntry]) {
        if let Some(listed) = &mut self.listed {
            listed.extend(entries.iter().map(|e| e.key.clone()));
            if listed.len() > KNOWN_KEYS {
                self.listed = None;
            }
        }
        let last = entries.last().map(|e| e.key.clone());
        let full = entries.len() >= MAX_ENTRIES;
        if let Some(rescan) = &self.rescan
            && (!full || last.as_ref().is_some_and(|last| *last >= rescan.target))
        {
            self.rescan = None;
        }
        if full {
            self.after = last;
            self.full_pages = self.full_pages.saturating_add(1);
        } else {
            self.after = None;
            self.full_pages = 0;
            if let Some(listed) = self.listed.take() {
                self.known.retain(|key, _| listed.contains(key));
            }
        }
    }
}

/// The consumer loop (§4.1): list, read, decide, publish, one exchange at a time.
pub struct Consumer<C, S, D> {
    clock: C,
    source: S,
    decider: D,
    source_authority: String,
    event: fn(&str),
    cursor: Cursor,
}

/// Outcome of one key's exchanges.
enum Step {
    Settled(Settle),
    /// A `429` or `503`: stop this page and back off.
    Throttled,
}

impl<C: ReportClock, S: CommandSource, D: Decider> Consumer<C, S, D> {
    pub fn new(clock: C, source: S, decider: D, source_authority: String, event: fn(&str)) -> Self {
        let cursor = Cursor::new(clock.elapsed());
        Self {
            clock,
            source,
            decider,
            source_authority,
            event,
            cursor,
        }
    }

    /// Runs until the source authority is outside the contract grammar.
    pub async fn run(mut self) {
        let mut backoff = POLL;
        loop {
            let after = self.cursor.next_request(self.clock.elapsed());
            let Ok(body) = encode_list(&self.source_authority, after.as_ref()) else {
                (self.event)("event=character_commands state=refused reason=request");
                return;
            };
            let started = self.clock.elapsed();
            let page = self
                .source
                .exchange(Operation::ListPendingCharacterCommandsV1, &body)
                .await
                .and_then(|raw| {
                    decode_pending(&raw, after.as_ref()).map_err(|_| Failure::InvalidResponse)
                });
            let entries = match page {
                Ok(entries) => entries,
                Err(failure) => {
                    self.log(
                        "ListPendingCharacterCommandsV1",
                        None,
                        failure.label(),
                        started,
                    );
                    backoff = if failure.backs_off() {
                        backoff.saturating_mul(2).clamp(POLL, MAX_BACKOFF)
                    } else {
                        POLL
                    };
                    self.clock.sleep(backoff).await;
                    continue;
                }
            };
            if self.cursor.observe(&entries) {
                (self.event)("event=character_commands alarm=rescan_backlog");
            }
            let mut throttled = false;
            for entry in &entries {
                if !self.cursor.due(&entry.key, self.clock.elapsed()) {
                    continue;
                }
                match self.process(entry).await {
                    Step::Settled(settle) => {
                        self.cursor
                            .settle(entry.key.clone(), settle, self.clock.elapsed());
                    }
                    Step::Throttled => {
                        throttled = true;
                        break;
                    }
                }
            }
            if throttled {
                backoff = backoff.saturating_mul(2).clamp(POLL, MAX_BACKOFF);
                self.clock.sleep(backoff).await;
                continue;
            }
            backoff = POLL;
            let short = entries.len() < MAX_ENTRIES;
            self.cursor.advance(&entries);
            if short {
                self.clock.sleep(POLL).await;
            }
        }
    }

    fn log(&self, operation: &str, command: Option<&str>, result: &str, started: Duration) {
        let elapsed = self.clock.elapsed().saturating_sub(started).as_millis();
        (self.event)(&format!(
            "event=character_commands operation={operation} command={} result={result} elapsed_ms={elapsed}",
            command.unwrap_or("-"),
        ));
    }

    async fn process(&mut self, entry: &PendingEntry) -> Step {
        let command = entry.command.as_str();
        let operation_id = entry.key.operation_id.as_str();
        let started = self.clock.elapsed();
        let intent;
        let input = if command == CREATE_COMMAND {
            let Ok(body) = encode_read(&self.source_authority, command, operation_id) else {
                return Step::Settled(Settle::Done);
            };
            match self
                .source
                .exchange(Operation::ReadCharacterCommandIntentV1, &body)
                .await
            {
                Ok(raw) => {
                    intent = raw;
                    Input::Intent(&intent)
                }
                Err(Failure::Conflict) => Input::ReadConflict,
                Err(Failure::Oversized) => Input::Oversized,
                Err(failure) => {
                    let alarm = if failure == Failure::NotFound {
                        "not_found alarm=intent_missing"
                    } else {
                        failure.label()
                    };
                    self.log(
                        "ReadCharacterCommandIntentV1",
                        Some(command),
                        alarm,
                        started,
                    );
                    return match failure {
                        Failure::NotFound => Step::Settled(Settle::Done),
                        f if f.backs_off() => Step::Throttled,
                        _ => Step::Settled(Settle::Retry),
                    };
                }
            }
        } else {
            Input::Unsupported
        };
        let decision = self.decider.decide(command, operation_id, input).await;
        let (receipt, result_code) = match decision {
            Decision::Publish {
                receipt,
                result_code,
            } => (receipt, result_code),
            Decision::Conflict => {
                self.log(
                    "decide",
                    Some(command),
                    "CHAR_CMD_OPERATION_CONFLICT alarm=receipt_conflict",
                    started,
                );
                return Step::Settled(Settle::Done);
            }
            Decision::Transient => {
                self.log("decide", Some(command), "transient", started);
                return Step::Settled(Settle::Retry);
            }
        };
        let Ok(body) = std::str::from_utf8(&receipt) else {
            return Step::Settled(Settle::Done);
        };
        let published = self
            .source
            .exchange(Operation::PublishCharacterCommandReceiptV1, body)
            .await
            .and_then(|raw| decode_receipt_response(&raw).map_err(|_| Failure::InvalidResponse));
        let result = match published {
            Ok(()) if result_code == "CHAR_CMD_OPERATION_CONFLICT" => {
                format!("{result_code} receipt=accepted alarm=operation_conflict")
            }
            Ok(()) => format!("{result_code} receipt=accepted"),
            Err(f @ (Failure::NotFound | Failure::Conflict)) => {
                format!("{result_code} receipt={} alarm=receipt_refused", f.label())
            }
            Err(f) => format!("{result_code} receipt={}", f.label()),
        };
        self.log(
            "PublishCharacterCommandReceiptV1",
            Some(command),
            &result,
            started,
        );
        match published {
            Ok(()) => Step::Settled(Settle::Done),
            Err(Failure::NotFound | Failure::Conflict) => Step::Settled(Settle::Stopped),
            Err(f) if f.backs_off() => Step::Throttled,
            Err(_) => Step::Settled(Settle::Retry),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "character_commands_tests.rs"]
mod tests;
