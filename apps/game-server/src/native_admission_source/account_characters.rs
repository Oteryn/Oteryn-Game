//! `ListCharactersForAccount` producer (`oteryn-game-list-characters-for-account-v1`
//! §2–§5, §7, §8).
//!
//! The Character Authority publisher pushes per-account snapshots and a
//! liveness watermark to Platform over its own purpose-bound mTLS identity.
//! Publication never affects gameplay, admission or the Character mutation
//! that caused it: every failure is only "not delivered", and the undelivered
//! outbox row holds the watermark back so Platform's feed goes stale.

use super::{
    SourceError, TransientCapacity,
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
    runtime_status::{NotDelivered, ReportClock, canonical_uuid, purpose_descriptor, refusal},
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use std::{future::Future, time::Duration};

/// `LCA-CHARACTERS`: wire bound, not a product slot quota.
pub const MAX_CHARACTERS: usize = 64;
/// `LCA-REQUEST-BYTES`.
pub const SNAPSHOT_BYTES: usize = 16_384;
/// `LCA-WATERMARK-BYTES`.
pub const WATERMARK_BYTES: usize = 512;
/// Response bound (§3).
pub const RESPONSE_BYTES: usize = 256;
/// Watermark period. §5.1 requires at most 10 s between watermarks; one
/// snapshot exchange (at most 6 s) may run between two due times.
pub const WATERMARK_PERIOD: Duration = Duration::from_secs(3);
/// Wait before retrying after an empty outbox or a failed delivery.
pub const IDLE: Duration = Duration::from_secs(1);
/// Compiled namespace of the projection descriptor; never a configured
/// source authority, so no other purpose's descriptor matches it.
pub const PURPOSE: &str = "OTERYN_GAME_ACCOUNT_CHARACTERS_PROJECTION";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Availability {
    Available,
    Unavailable,
}

/// `CharacterSummary` v1 (§2.1), in wire member order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CharacterSummary {
    pub character_id: String,
    pub world_id: String,
    pub name: String,
    pub availability: Availability,
}

/// One account's complete current list, read in one database snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSnapshot {
    pub account_id: String,
    pub projection_epoch: u64,
    pub projection_revision: u64,
    pub source_observed_at: i64,
    pub characters: Vec<CharacterSummary>,
}

/// The facts the §5.1 watermark is computed from, read on the database clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatermarkFacts {
    pub projection_epoch: u64,
    /// `created_at` (Unix ms) of the oldest undelivered outbox row.
    pub oldest_undelivered_ms: Option<i64>,
    pub now_ms: i64,
}

#[derive(Serialize)]
struct SnapshotWire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    account_id: &'a str,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    characters: &'a [CharacterSummary],
}

#[derive(Serialize)]
struct WatermarkWire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    projection_epoch: String,
    complete_through: String,
    observed_at: String,
}

fn source_authority_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}

/// §2.1 name rule. The producer emits the printable-ASCII subset, which is
/// NFC and holds no control (Cc) or format (Cf) character; `"` and `\` are
/// refused so the name serializes without escapes. Anything else refuses the
/// snapshot rather than being altered.
#[must_use]
pub fn name_valid(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| (0x20..=0x7e).contains(&b) && b != b'"' && b != b'\\')
}

/// Exact §4 snapshot body. Refuses anything outside the contract grammar.
pub fn encode_snapshot(
    source_authority: &str,
    snapshot: &AccountSnapshot,
) -> Result<String, SourceError> {
    let characters = &snapshot.characters;
    let valid = source_authority_valid(source_authority)
        && canonical_uuid(&snapshot.account_id, Some(b'7'))
        && snapshot.projection_epoch != 0
        && snapshot.projection_revision != 0
        && snapshot.source_observed_at >= 0
        && characters.len() <= MAX_CHARACTERS
        && characters
            .windows(2)
            .all(|w| w[0].character_id < w[1].character_id)
        && characters.iter().all(|c| {
            canonical_uuid(&c.character_id, Some(b'7'))
                && canonical_uuid(&c.world_id, Some(b'7'))
                && name_valid(&c.name)
        });
    if !valid {
        return Err(SourceError::InvalidInput);
    }
    let body = serde_json::to_string(&SnapshotWire {
        contract_version: 1,
        operation: "PublishAccountCharactersV1",
        source_authority,
        account_id: &snapshot.account_id,
        projection_epoch: snapshot.projection_epoch.to_string(),
        projection_revision: snapshot.projection_revision.to_string(),
        source_observed_at: snapshot.source_observed_at.to_string(),
        characters,
    })
    .map_err(|_| SourceError::InvalidInput)?;
    if body.len() > SNAPSHOT_BYTES || body.contains('\\') {
        return Err(SourceError::CapacityExceeded);
    }
    Ok(body)
}

/// §5.1 `complete_through` in Unix seconds: the latest second T such that
/// every change committed at or before T is delivered. It stays below the
/// oldest undelivered row (which includes every queued resync row) and at
/// least `max_transaction` behind now, so a slow transaction that commits
/// late is still covered. `None` when no such second is representable.
#[must_use]
pub fn complete_through(facts: &WatermarkFacts, max_transaction: Duration) -> Option<i64> {
    let slack = i64::try_from(max_transaction.as_millis()).ok()?;
    let mut through_ms = facts.now_ms.checked_sub(slack)?;
    if let Some(oldest) = facts.oldest_undelivered_ms {
        through_ms = through_ms.min(oldest.checked_sub(1)?);
    }
    let seconds = through_ms.div_euclid(1000);
    (seconds >= 0).then_some(seconds)
}

/// Exact §4 watermark body.
pub fn encode_watermark(
    source_authority: &str,
    facts: &WatermarkFacts,
    max_transaction: Duration,
) -> Result<String, SourceError> {
    let through = complete_through(facts, max_transaction).ok_or(SourceError::InvalidInput)?;
    if !source_authority_valid(source_authority) || facts.projection_epoch == 0 {
        return Err(SourceError::InvalidInput);
    }
    let body = serde_json::to_string(&WatermarkWire {
        contract_version: 1,
        operation: "PublishProjectionWatermarkV1",
        source_authority,
        projection_epoch: facts.projection_epoch.to_string(),
        complete_through: through.to_string(),
        observed_at: facts.now_ms.div_euclid(1000).to_string(),
    })
    .map_err(|_| SourceError::InvalidInput)?;
    if body.len() > WATERMARK_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    Ok(body)
}

/// Platform's definite acknowledgement (§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Delivery {
    Accepted,
    Superseded,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ack {
    contract_version: u8,
    result: Delivery,
}

/// Exact success body; unknown, duplicate, missing or `null` members refuse.
pub fn decode_response(raw: &[u8]) -> Result<Delivery, SourceError> {
    if raw.len() > RESPONSE_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    match serde_json::from_slice::<Ack>(raw) {
        Ok(Ack {
            contract_version: 1,
            result,
        }) => Ok(result),
        _ => Err(SourceError::InvalidInput),
    }
}

/// Projection client identity (§3): its own certificate and key, never those
/// of the evidence, intent or runtime-status purposes.
pub struct ProjectionDescriptor(ProducerDescriptor);

impl ProjectionDescriptor {
    /// `other_purposes` are the certificate chains of every other Platform
    /// identity this host holds; none may share the projection public key.
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

/// Where publications go: Platform over mTLS, or a test sink.
pub trait ProjectionSink: Send {
    fn send(
        &mut self,
        operation: Operation,
        body: &str,
    ) -> impl Future<Output = Result<Delivery, NotDelivered>> + Send;
}

/// The production sink: one exchange at a time on its own capacity (`LCA-INFLIGHT`).
pub struct MtlsSink {
    descriptor: ProjectionDescriptor,
    capacity: TransientCapacity,
}

impl MtlsSink {
    #[must_use]
    pub const fn new(descriptor: ProjectionDescriptor) -> Self {
        Self {
            descriptor,
            capacity: TransientCapacity::new(),
        }
    }
}

impl ProjectionSink for MtlsSink {
    async fn send(&mut self, operation: Operation, body: &str) -> Result<Delivery, NotDelivered> {
        let mut permit = self
            .capacity
            .try_queue()
            .map_err(|_| NotDelivered::Unavailable)?;
        permit
            .try_activate()
            .map_err(|_| NotDelivered::Unavailable)?;
        let (status, raw) =
            http1_mtls::exchange_with_status(&self.descriptor.0, operation, body, &mut permit)
                .await
                .map_err(|_| NotDelivered::Unavailable)?;
        if status == 200 {
            return decode_response(&raw).map_err(|_| NotDelivered::InvalidResponse);
        }
        Err(refusal(status, &raw))
    }
}

/// The Character Authority side of the projection (§5).
pub trait ProjectionStore: Send {
    /// The current snapshot of the account with the oldest undelivered change.
    fn next_snapshot(&mut self)
    -> impl Future<Output = Result<Option<AccountSnapshot>, ()>> + Send;
    /// Clear that account's outbox rows up to the acknowledged pair.
    fn clear(
        &mut self,
        account_id: &str,
        projection_epoch: u64,
        projection_revision: u64,
    ) -> impl Future<Output = Result<(), ()>> + Send;
    fn watermark_facts(&mut self) -> impl Future<Output = Result<WatermarkFacts, ()>> + Send;
}

/// One publisher per Character Authority: publications are sequential, so at
/// most one is in flight.
pub struct Publisher<C, S, K> {
    pub clock: C,
    pub store: S,
    pub sink: K,
    pub source_authority: String,
    /// Maximum Character transaction duration (§5.1).
    pub max_transaction: Duration,
    watermark_due: Duration,
}

fn log(operation: &str, class: &str, elapsed: Duration) {
    // §7: operation, result class and timing only; never an AccountId or name.
    eprintln!(
        "oteryn-game-server event=account_characters_projection operation={operation} result={class} elapsed_ms={}",
        elapsed.as_millis()
    );
}

fn class(result: &Result<Delivery, NotDelivered>) -> &'static str {
    match result {
        Ok(Delivery::Accepted) => "accepted",
        Ok(Delivery::Superseded) => "superseded",
        Err(not_delivered) => not_delivered.class(),
    }
}

impl<C: ReportClock, S: ProjectionStore, K: ProjectionSink> Publisher<C, S, K> {
    pub const fn new(
        clock: C,
        store: S,
        sink: K,
        source_authority: String,
        max_transaction: Duration,
    ) -> Self {
        Self {
            clock,
            store,
            sink,
            source_authority,
            max_transaction,
            watermark_due: Duration::ZERO,
        }
    }

    /// The watermark when due, then at most one snapshot. Returns whether a
    /// snapshot was acknowledged and cleared.
    pub async fn step(&mut self) -> bool {
        if self.clock.elapsed() >= self.watermark_due {
            self.watermark_due = self.clock.elapsed() + WATERMARK_PERIOD;
            let started = self.clock.elapsed();
            let result = match self.store.watermark_facts().await {
                Err(()) => Err(NotDelivered::Unavailable),
                Ok(facts) => {
                    match encode_watermark(&self.source_authority, &facts, self.max_transaction) {
                        Err(_) => Err(NotDelivered::InvalidReport),
                        Ok(body) => {
                            self.sink
                                .send(Operation::PublishProjectionWatermarkV1, &body)
                                .await
                        }
                    }
                }
            };
            let elapsed = self.clock.elapsed().saturating_sub(started);
            log("PublishProjectionWatermarkV1", class(&result), elapsed);
        }
        let Ok(Some(snapshot)) = self.store.next_snapshot().await else {
            return false;
        };
        let started = self.clock.elapsed();
        let result = match encode_snapshot(&self.source_authority, &snapshot) {
            // An unpublishable snapshot stays queued, so the watermark stalls
            // and Platform refuses issuance (fail toward no issuance).
            Err(_) => Err(NotDelivered::InvalidReport),
            Ok(body) => {
                self.sink
                    .send(Operation::PublishAccountCharactersV1, &body)
                    .await
            }
        };
        let elapsed = self.clock.elapsed().saturating_sub(started);
        log("PublishAccountCharactersV1", class(&result), elapsed);
        result.is_ok()
            && self
                .store
                .clear(
                    &snapshot.account_id,
                    snapshot.projection_epoch,
                    snapshot.projection_revision,
                )
                .await
                .is_ok()
    }

    /// Publish until the task is dropped.
    pub async fn run(mut self) {
        loop {
            if !self.step().await {
                let wait = self
                    .watermark_due
                    .saturating_sub(self.clock.elapsed())
                    .min(IDLE);
                self.clock.sleep(wait).await;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "account_characters_tests.rs"]
mod tests;
