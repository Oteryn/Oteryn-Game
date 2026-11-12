//! One in-flight journal exchange; source storage owns fixed bytes and authority.
use super::*;
use crate::{
    TransientCapacity,
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use sha2::{Digest, Sha256};
use std::{future::Future, time::Duration};

pub const PURPOSE: &str = "OTERYN_GAME_JOURNAL_ITEMS";
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct JournalExportConfig {
    pub game_journal_items_export_enabled: bool,
    pub journal_progress_publish_enabled: bool,
}
/// Constructed only with a distinct journal identity using the existing verifier.
pub struct JournalDescriptor(ProducerDescriptor);
impl JournalDescriptor {
    pub fn new(
        endpoint: (String, u16),
        peer: String,
        roots: Vec<CertificateDer<'static>>,
        chain: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
        others: &[&[CertificateDer<'static>]],
    ) -> Result<Self, SourceError> {
        crate::runtime_status::purpose_descriptor(
            PURPOSE, endpoint, peer, roots, chain, key, others,
        )
        .map(Self)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Retry(Option<Duration>),
    Conflict,
    Quarantine,
}
pub trait JournalPeer {
    fn exchange(
        &mut self,
        operation: Operation,
        bytes: &[u8],
    ) -> impl Future<Output = Result<Vec<u8>, Failure>>;
}
pub struct MtlsJournalPeer {
    descriptor: JournalDescriptor,
    capacity: TransientCapacity,
    next_exchange: Option<tokio::time::Instant>,
}
impl MtlsJournalPeer {
    pub const fn new(descriptor: JournalDescriptor) -> Self {
        Self {
            descriptor,
            capacity: TransientCapacity::new(),
            next_exchange: None,
        }
    }
}
impl JournalPeer for MtlsJournalPeer {
    async fn exchange(&mut self, operation: Operation, bytes: &[u8]) -> Result<Vec<u8>, Failure> {
        if !matches!(
            operation,
            Operation::PublishGameJournalRecordV1 | Operation::ReadGameJournalCursorV1
        ) {
            return Err(Failure::Quarantine);
        }
        // Uniform spacing bounds the peer to 120 requests/minute without bursts.
        if let Some(at) = self.next_exchange {
            tokio::time::sleep_until(at).await;
        }
        self.next_exchange = Some(tokio::time::Instant::now() + Duration::from_millis(500));
        let body = std::str::from_utf8(bytes).map_err(|_| Failure::Quarantine)?;
        let mut permit = self
            .capacity
            .try_queue()
            .map_err(|_| Failure::Retry(None))?;
        permit.try_activate().map_err(|_| Failure::Retry(None))?;
        let (status, body, retry) =
            http1_mtls::exchange_store(&self.descriptor.0, operation, body, &mut permit)
                .await
                .map_err(|error| match error {
                    SourceError::InvalidDescriptor => Failure::Quarantine,
                    _ => Failure::Retry(None),
                })?;
        classify_response(status, body, retry)
    }
}
fn classify_response(
    status: u16,
    body: Vec<u8>,
    retry: Option<Duration>,
) -> Result<Vec<u8>, Failure> {
    match status {
        200 => Ok(body),
        409 => Err(Failure::Conflict),
        400 | 401 | 413 => Err(Failure::Quarantine),
        _ => Err(Failure::Retry(retry)),
    }
}
#[derive(Debug, Clone)]
pub struct StoredPublication {
    pub bytes: Vec<u8>,
    pub sha256: [u8; 32],
}
/// Implementations revalidate live fences in each short storage transaction.
/// `after_cursor` binds a nonzero cursor to its retained exact record ID, updates
/// local ACK evidence, and returns the next immutable record even if previously ACKed.
/// Missing retained evidence is an error, never an implicit empty history.
pub trait JournalOutbox {
    fn current(
        &self,
        stream: &ReadGameJournalCursorV1,
    ) -> impl Future<Output = Result<bool, SourceError>>;
    /// An unknown remote stream is not a zero cursor. Only a locally registered
    /// cut with retained revision-1 bytes can bootstrap by exact replay.
    fn first_for_replay(
        &self,
        stream: &ReadGameJournalCursorV1,
    ) -> impl Future<Output = Result<Option<StoredPublication>, SourceError>>;
    fn after_cursor(
        &self,
        stream: &ReadGameJournalCursorV1,
        cursor: &ReadGameJournalCursorV1Response,
    ) -> impl Future<Output = Result<Option<StoredPublication>, SourceError>>;
    fn acknowledge(
        &self,
        stream: &ReadGameJournalCursorV1,
        record: &PublishGameJournalRecordV1,
    ) -> impl Future<Output = Result<(), SourceError>>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Disabled,
    Idle,
    Accepted,
    RetryAfter(Duration),
    Quarantined,
}
pub struct Publisher<P> {
    config: JournalExportConfig,
    peer: P,
    authority: String,
    attempt: u64,
    last_progress: tokio::time::Instant,
    last_diagnostic: tokio::time::Instant,
}
impl<P: JournalPeer> Publisher<P> {
    pub fn new(
        config: JournalExportConfig,
        peer: P,
        authority: String,
    ) -> Result<Self, SourceError> {
        if !source_authority_valid(&authority) {
            return Err(SourceError::InvalidInput);
        }
        Ok(Self {
            config,
            peer,
            authority,
            attempt: 0,
            last_progress: tokio::time::Instant::now(),
            last_diagnostic: tokio::time::Instant::now(),
        })
    }
    /// Restore the durable per-stream retry sequence after selection/restart.
    pub fn restore_retry_attempt(&mut self, attempt: u64) {
        self.attempt = attempt;
    }
    pub fn retry_attempt(&self) -> u64 {
        self.attempt
    }
    fn retry(&mut self, key: &[u8], specified: Option<Duration>) -> Step {
        self.attempt = self.attempt.wrapping_add(1);
        let mut hash = Sha256::new();
        hash.update(key);
        hash.update(self.attempt.to_be_bytes());
        let ceiling = (1_u64 << self.attempt.saturating_sub(1).min(4)).min(10);
        // Equal jitter preserves exponential growth while remaining within 1-10s.
        let floor = ceiling.div_ceil(2);
        let jitter = Duration::from_millis(
            floor * 1000 + u64::from(hash.finalize()[0]) * (ceiling - floor) * 1000 / 255,
        );
        let now = tokio::time::Instant::now();
        if now.duration_since(self.last_progress) >= Duration::from_secs(60)
            && now.duration_since(self.last_diagnostic) >= Duration::from_secs(60)
        {
            eprintln!("event=game_journal_stalled reason=no_progress");
            self.last_diagnostic = now;
        }
        Step::RetryAfter(specified.map_or(jitter, |delay| delay.max(jitter)))
    }

    pub async fn step(
        &mut self,
        stream: &ReadGameJournalCursorV1,
        store: &impl JournalOutbox,
    ) -> Step {
        if !self.config.game_journal_items_export_enabled {
            return Step::Disabled;
        }
        if stream.source_authority != self.authority {
            return Step::Quarantined;
        }
        let Ok(key) = stream.encode() else {
            return Step::Quarantined;
        };
        let live = match store.current(stream).await {
            Ok(live) => live,
            Err(_) => return self.retry(&key, None),
        };
        if !live {
            return Step::Quarantined;
        }
        let cursor = match self
            .peer
            .exchange(Operation::ReadGameJournalCursorV1, &key)
            .await
        {
            Ok(bytes) => match ReadGameJournalCursorV1Response::decode(&bytes) {
                Ok(value) => Some(value),
                Err(_) => return self.retry(&key, None),
            },
            Err(Failure::Retry(delay)) => return self.retry(&key, delay),
            Err(Failure::Conflict) => None,
            Err(Failure::Quarantine) => return Step::Quarantined,
        };
        let bootstrapping = cursor.is_none();
        let selected = match &cursor {
            Some(cursor) => store.after_cursor(stream, cursor).await,
            None => store.first_for_replay(stream).await,
        };
        let publication = match selected {
            Ok(Some(value)) => value,
            Ok(None) => {
                return if bootstrapping {
                    Step::Quarantined
                } else {
                    self.attempt = 0;
                    self.last_progress = tokio::time::Instant::now();
                    Step::Idle
                };
            }
            Err(SourceError::InvalidInput | SourceError::InvalidDescriptor) => {
                return Step::Quarantined;
            }
            Err(_) => return self.retry(&key, None),
        };
        if Sha256::digest(&publication.bytes).as_slice() != publication.sha256 {
            return Step::Quarantined;
        }
        let Ok(record) = PublishGameJournalRecordV1::decode(&publication.bytes) else {
            return Step::Quarantined;
        };
        let expected = match &cursor {
            Some(cursor) => cursor
                .after_source_revision
                .parse::<u64>()
                .ok()
                .and_then(|v| v.checked_add(1)),
            None => Some(1),
        };
        if counter(&record.source_revision) != expected || !same_stream(stream, &record) {
            return Step::Quarantined;
        }
        let live = match store.current(stream).await {
            Ok(live) => live,
            Err(_) => return self.retry(&key, None),
        };
        if !live {
            return Step::Quarantined;
        }
        match self
            .peer
            .exchange(Operation::PublishGameJournalRecordV1, &publication.bytes)
            .await
        {
            Ok(bytes) => {
                if PublishGameJournalRecordV1Response::decode(&bytes).is_err() {
                    return self.retry(&key, None);
                }
                if store.acknowledge(stream, &record).await.is_err() {
                    return self.retry(&key, None);
                }
                self.attempt = 0;
                self.last_progress = tokio::time::Instant::now();
                Step::Accepted
            }
            // The next step always rereads the cursor, including after a lost ACK or gap.
            Err(Failure::Conflict) if bootstrapping => Step::Quarantined,
            Err(Failure::Conflict) => self.retry(&key, None),
            Err(Failure::Retry(delay)) => self.retry(&key, delay),
            Err(Failure::Quarantine) => Step::Quarantined,
        }
    }
}
fn same_stream(s: &ReadGameJournalCursorV1, r: &PublishGameJournalRecordV1) -> bool {
    s.source_authority == r.source_authority
        && s.account_id == r.account_id
        && s.world_id == r.world_id
        && s.character_id == r.character_id
        && s.character_revision == r.character_revision
        && s.from_projection_revision == r.from_projection_revision
        && s.to_projection_revision == r.to_projection_revision
        && s.projection_epoch == r.projection_epoch
        && s.family == r.family
        && s.recovery_generation == r.recovery_generation
}
#[cfg(test)]
mod tests;
