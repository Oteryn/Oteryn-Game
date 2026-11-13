//! G-3 private companion: independent acknowledgement, shared LCFA epoch witness.
use crate::{
    SourceError, TransientCapacity,
    account_characters::{
        EpochFence, IDLE, MAX_WATERMARK_GAP, WATERMARK_PERIOD, WatermarkFacts, complete_through,
        within,
    },
    character_services_v2::{
        AcknowledgementV2, PublishCharacterServiceStateV2, PublishCharacterServiceWatermarkV2,
    },
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
    runtime_status::{ReportClock, purpose_descriptor},
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde_json::{Value, json};
use std::{future::Future, time::Duration};
/// Dedicated compiled purpose, never an authority selected by a body.
pub const PURPOSE: &str = "OTERYN_GAME_CHARACTER_SERVICES_PROJECTION";
#[derive(Clone)]
pub struct Descriptor(ProducerDescriptor);
impl Descriptor {
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
#[derive(Clone, PartialEq)]
pub struct Snapshot {
    pub account_id: String,
    pub character_id: String,
    pub epoch: u64,
    pub revision: u64,
    pub facts: Value,
}
const FIELDS: [&str; 13] = [
    "account_id",
    "character_id",
    "projection_epoch",
    "projection_revision",
    "source_observed_at",
    "character_revision",
    "world_id",
    "name",
    "sex",
    "lifecycle",
    "deletion_operation_id",
    "deletion_scheduled_at",
    "deletion_effective_at",
];
impl Snapshot {
    pub fn encode(&self, source: &str) -> Result<String, SourceError> {
        if self.facts["account_id"].as_str() != Some(self.account_id.as_str())
            || self.facts["character_id"].as_str() != Some(self.character_id.as_str())
            || self.facts["projection_epoch"] != json!(self.epoch.to_string())
            || self.facts["projection_revision"] != json!(self.revision.to_string())
        {
            return Err(SourceError::InvalidInput);
        }
        let mut fields = head("PublishCharacterServiceStateV2", source);
        for name in &FIELDS {
            fields.push((
                (*name).into(),
                self.facts
                    .get(*name)
                    .cloned()
                    .ok_or(SourceError::InvalidInput)?,
            ));
        }
        let bytes = PublishCharacterServiceStateV2::from_fields(fields)?.encode()?;
        String::from_utf8(bytes).map_err(|_| SourceError::InvalidInput)
    }
}
fn head(operation: &str, source: &str) -> Vec<(String, Value)> {
    vec![
        ("contract_version".into(), json!(2)),
        ("operation".into(), json!(operation)),
        ("source_authority".into(), json!(source)),
    ]
}
fn watermark(
    source: &str,
    facts: &WatermarkFacts,
    max_transaction: Duration,
) -> Result<String, SourceError> {
    let through = complete_through(facts, max_transaction)
        .filter(|v| *v > 0)
        .ok_or(SourceError::InvalidInput)?;
    let mut fields = head("PublishCharacterServiceWatermarkV2", source);
    fields.extend([
        (
            "projection_epoch".into(),
            json!(facts.projection_epoch.to_string()),
        ),
        ("complete_through".into(), json!(through.to_string())),
        (
            "observed_at".into(),
            json!(facts.now_ms.div_euclid(1000).to_string()),
        ),
    ]);
    String::from_utf8(PublishCharacterServiceWatermarkV2::from_fields(fields)?.encode()?)
        .map_err(|_| SourceError::InvalidInput)
}
/// Persistent state is owned by Character Authority, not by transport retries.
pub trait Store: Send {
    fn next(&mut self) -> impl Future<Output = Result<Option<Snapshot>, ()>> + Send;
    fn acknowledge(&mut self, snapshot: &Snapshot) -> impl Future<Output = Result<(), ()>> + Send;
    fn quarantine(&mut self, snapshot: &Snapshot) -> impl Future<Output = Result<(), ()>> + Send;
    fn quarantine_feed(&mut self, epoch: u64) -> impl Future<Output = Result<(), ()>> + Send;
    fn facts(&mut self) -> impl Future<Output = Result<WatermarkFacts, ()>> + Send;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Retry(Option<Duration>),
    Quarantine,
}
pub trait Peer: Send {
    fn send(
        &mut self,
        operation: Operation,
        body: &str,
    ) -> impl Future<Output = Result<(), Failure>> + Send;
}
pub struct MtlsPeer {
    descriptor: Descriptor,
    capacity: TransientCapacity,
}
impl MtlsPeer {
    pub const fn new(descriptor: Descriptor) -> Self {
        Self {
            descriptor,
            capacity: TransientCapacity::new(),
        }
    }
}
impl Peer for MtlsPeer {
    async fn send(&mut self, operation: Operation, body: &str) -> Result<(), Failure> {
        if !matches!(
            operation,
            Operation::PublishCharacterServiceStateV2
                | Operation::PublishCharacterServiceWatermarkV2
        ) {
            return Err(Failure::Quarantine);
        }
        let mut permit = self
            .capacity
            .try_queue()
            .map_err(|_| Failure::Retry(None))?;
        permit.try_activate().map_err(|_| Failure::Retry(None))?;
        let (status, body, retry) =
            http1_mtls::exchange_store(&self.descriptor.0, operation, body, &mut permit)
                .await
                .map_err(|_| Failure::Retry(None))?;
        if status == 200 {
            return AcknowledgementV2::decode(&body)
                .map(|_| ())
                .map_err(|_| Failure::Retry(None));
        }
        match status {
            400 | 401 | 409 | 413 => Err(Failure::Quarantine),
            _ => Err(Failure::Retry(retry)),
        }
    }
}
fn log(operation: &'static str, result: &'static str, elapsed: Duration) {
    use oteryn_error_codes::{FieldValue, Level, Line};
    oteryn_error_codes::emit(
        "oteryn-game-server",
        &Line::new(Level::Info, "admission", "character_services_projection")
            .field("operation", FieldValue::Token(operation))
            .field("result", FieldValue::Token(result))
            .field(
                "ms",
                FieldValue::U64(elapsed.as_millis().try_into().unwrap_or(u64::MAX)),
            ),
    );
}
/// One in-flight exchange. LCFA remains the sole writer of the shared epoch file.
pub struct Publisher<C, S, P, F> {
    pub clock: C,
    pub store: S,
    pub peer: P,
    pub fence: F,
    pub source: String,
    pub max_transaction: Duration,
    watermark_started: Duration,
    watermark_due: Duration,
    not_before: Duration,
    retry: Duration,
}
impl<C: ReportClock, S: Store, P: Peer, F: EpochFence> Publisher<C, S, P, F> {
    pub fn new(
        clock: C,
        store: S,
        peer: P,
        fence: F,
        source: String,
        max_transaction: Duration,
    ) -> Self {
        Self {
            clock,
            store,
            peer,
            fence,
            source,
            max_transaction,
            watermark_started: Duration::ZERO,
            watermark_due: Duration::ZERO,
            not_before: Duration::ZERO,
            retry: IDLE,
        }
    }
    fn admit(&mut self, epoch: u64) -> bool {
        epoch > 0 && self.fence.read().ok() == Some(epoch)
    }
    fn failed(&mut self, failure: Failure) {
        if let Failure::Retry(header) = failure {
            self.not_before = self
                .clock
                .elapsed()
                .saturating_add(header.unwrap_or(self.retry).max(self.retry));
            self.retry = self
                .retry
                .saturating_mul(2)
                .min(crate::character_commands::MAX_BACKOFF);
        }
    }
    pub async fn step(&mut self) -> bool {
        if self.clock.elapsed() < self.not_before {
            return false;
        }
        if self.clock.elapsed() >= self.watermark_due {
            let started = self.clock.elapsed();
            self.watermark_started = started;
            self.watermark_due = started + WATERMARK_PERIOD;
            let mut epoch = None;
            let outcome = match self.store.facts().await {
                Ok(facts) if self.admit(facts.projection_epoch) => {
                    epoch = Some(facts.projection_epoch);
                    match watermark(&self.source, &facts, self.max_transaction) {
                        Ok(body) => within(
                            &self.clock,
                            (started + MAX_WATERMARK_GAP).saturating_sub(self.clock.elapsed()),
                            self.peer
                                .send(Operation::PublishCharacterServiceWatermarkV2, &body),
                        )
                        .await
                        .unwrap_or(Err(Failure::Retry(None))),
                        Err(_) => Err(Failure::Retry(None)),
                    }
                }
                _ => Err(Failure::Retry(None)),
            };
            log(
                "PublishCharacterServiceWatermarkV2",
                if outcome.is_ok() {
                    "accepted"
                } else {
                    "pending"
                },
                self.clock.elapsed().saturating_sub(started),
            );
            if let Err(failure) = outcome {
                if failure == Failure::Quarantine
                    && let Some(epoch) = epoch
                {
                    let _ = self.store.quarantine_feed(epoch).await;
                }
                self.failed(failure);
                return false;
            }
        }
        let end = self.watermark_started + MAX_WATERMARK_GAP;
        if self.clock.elapsed() + self.max_transaction.saturating_mul(2) >= end {
            return false;
        }
        let Ok(Some(snapshot)) = self.store.next().await else {
            return false;
        };
        if !self.admit(snapshot.epoch) {
            return false;
        }
        let body = match snapshot.encode(&self.source) {
            Ok(body) => body,
            Err(_) => {
                let _ = self.store.quarantine(&snapshot).await;
                return false;
            }
        };
        let started = self.clock.elapsed();
        let budget = end.saturating_sub(started + self.max_transaction);
        let outcome = within(
            &self.clock,
            budget,
            self.peer
                .send(Operation::PublishCharacterServiceStateV2, &body),
        )
        .await
        .unwrap_or(Err(Failure::Retry(None)));
        log(
            "PublishCharacterServiceStateV2",
            if outcome.is_ok() {
                "accepted"
            } else {
                "pending"
            },
            self.clock.elapsed().saturating_sub(started),
        );
        match outcome {
            Ok(()) => {
                self.retry = IDLE;
                self.store.acknowledge(&snapshot).await.is_ok()
            }
            Err(Failure::Quarantine) => {
                let _ = self.store.quarantine(&snapshot).await;
                false
            }
            Err(failure) => {
                self.failed(failure);
                false
            }
        }
    }
    pub async fn run(mut self) {
        loop {
            if !self.step().await {
                let until = self
                    .watermark_due
                    .max(self.not_before)
                    .saturating_sub(self.clock.elapsed());
                self.clock.sleep(until.min(IDLE)).await;
            }
        }
    }
}
#[cfg(test)]
#[path = "character_services_projection_tests.rs"]
mod tests;
