//! Purpose-separated Game-initiated HTTP publisher. The store owns all truth.
use super::*;
use crate::{
    SourceError, TransientCapacity,
    account_characters::{Delivery, EpochFence, decode_response},
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
    runtime_status::{NotDelivered, ReportClock, purpose_descriptor, refusal},
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};
pub const PURPOSE: &str = "OTERYN_GAME_WORLD_EVENT_GUILD_SEASON";
#[derive(Clone)]
pub struct LeagueDescriptor(ProducerDescriptor, String);
impl LeagueDescriptor {
    pub fn public_key_id(&self) -> &str {
        &self.1
    }
    pub fn new(
        endpoint: (String, u16),
        peer: String,
        roots: Vec<CertificateDer<'static>>,
        chain: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
        others: &[&[CertificateDer<'static>]],
    ) -> std::result::Result<Self, SourceError> {
        let key_id = format!(
            "sha256:{}",
            digest(&crate::runtime_status::leaf_spki(&chain)?)
        );
        purpose_descriptor(PURPOSE, endpoint, peer, roots, chain, key, others)
            .map(|d| Self(d, key_id))
    }
}
pub trait LeagueStore {
    fn current(
        &self,
        family: &str,
        epoch: u64,
        token: &str,
    ) -> impl Future<Output = std::result::Result<bool, ()>>;
    fn produce(&self) -> impl Future<Output = std::result::Result<(), ()>>;
    fn next(
        &self,
        family: &str,
    ) -> impl Future<Output = std::result::Result<Option<Publication>, ()>>;
    fn failed(
        &self,
        p: &Publication,
        error: NotDelivered,
    ) -> impl Future<Output = std::result::Result<(), ()>>;
    fn release_watermark(
        &self,
        p: &WatermarkDelivery,
    ) -> impl Future<Output = std::result::Result<(), ()>>;
    fn ack(&self, publication: &Publication) -> impl Future<Output = std::result::Result<(), ()>>;
    fn watermark(
        &self,
        family: &str,
    ) -> impl Future<Output = std::result::Result<Option<WatermarkDelivery>, ()>>;
}
pub trait LeagueSink {
    fn send(
        &mut self,
        operation: Operation,
        body: &str,
    ) -> impl Future<Output = std::result::Result<Delivery, NotDelivered>>;
}
pub struct MtlsLeagueSink {
    descriptor: LeagueDescriptor,
    capacity: TransientCapacity,
}
impl MtlsLeagueSink {
    pub fn new(descriptor: LeagueDescriptor) -> Self {
        Self {
            descriptor,
            capacity: TransientCapacity::new(),
        }
    }
}
impl LeagueSink for MtlsLeagueSink {
    async fn send(
        &mut self,
        operation: Operation,
        body: &str,
    ) -> std::result::Result<Delivery, NotDelivered> {
        if !is_operation(operation) {
            return Err(NotDelivered::Malformed);
        }
        if body.len() > operation.request_bytes_max() {
            return Err(NotDelivered::Malformed);
        }
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
                .map_err(|e| match e {
                    SourceError::InvalidInput | SourceError::CapacityExceeded => {
                        NotDelivered::InvalidResponse
                    }
                    _ => NotDelivered::Unavailable,
                })?;
        if status == 200 {
            decode_response(&raw).map_err(|_| NotDelivered::InvalidResponse)
        } else {
            Err(refusal(status, &raw))
        }
    }
}
pub struct Publisher<C, S, K, F> {
    pub enabled: bool,
    pub clock: C,
    pub store: S,
    pub sink: K,
    pub fence: Arc<Mutex<F>>,
    pub family: &'static str,
    pub source_authority: String,
}
impl<C: ReportClock, S: LeagueStore, K: LeagueSink, F: EpochFence> Publisher<C, S, K, F> {
    fn admit(&self, epoch: u64) -> std::result::Result<(), NotDelivered> {
        let mut fence = self.fence.lock().map_err(|_| NotDelivered::Unavailable)?;
        let high = fence.read().map_err(|_| NotDelivered::Unavailable)?;
        if epoch < high {
            return Err(NotDelivered::Unavailable);
        }
        if epoch > high {
            fence
                .persist(epoch)
                .map_err(|_| NotDelivered::Unavailable)?
        }
        Ok(())
    }
    /// Re-reads current recovery authority on every attempt. An unacknowledged
    /// publication is immutable, so 429/503/restart retries retain identical bytes.
    pub async fn step(&mut self) -> std::result::Result<bool, NotDelivered> {
        if !self.enabled {
            return Err(NotDelivered::Unavailable);
        }
        let Some(p) = self
            .store
            .next(self.family)
            .await
            .map_err(|_| NotDelivered::Unavailable)?
        else {
            return Ok(false);
        };
        if digest(p.wire.as_bytes())
            != p.sha256
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        {
            let _ = self.store.failed(&p, NotDelivered::Conflict).await;
            return Err(NotDelivered::Conflict);
        }
        let snapshot = Snapshot::decode(&p.wire).map_err(|_| NotDelivered::Malformed)?;
        let (epoch, revision) = match &snapshot {
            Snapshot::Event(s) => (&s.projection_epoch, &s.projection_revision),
            Snapshot::Season(s) => (&s.projection_epoch, &s.projection_revision),
        };
        if decimal(epoch, false).map_err(|_| NotDelivered::Malformed)? != p.epoch
            || decimal(revision, false).map_err(|_| NotDelivered::Malformed)? != p.revision
        {
            let _ = self.store.failed(&p, NotDelivered::Conflict).await;
            return Err(NotDelivered::Conflict);
        }
        let authority = match &snapshot {
            Snapshot::Event(s) => &s.source_authority,
            Snapshot::Season(s) => &s.source_authority,
        };
        if authority != &self.source_authority {
            return Err(NotDelivered::Unauthenticated);
        }
        let operation = match snapshot {
            Snapshot::Event(_) if self.family == "world_event_outcome" => {
                Operation::PublishWorldEventOutcomeV1
            }
            Snapshot::Season(_) if self.family == "guild_season" => Operation::PublishGuildSeasonV1,
            _ => return Err(NotDelivered::Malformed),
        };
        self.admit(p.epoch)?;
        if !self
            .store
            .current(
                self.family,
                p.epoch,
                p.delivery_token
                    .as_deref()
                    .ok_or(NotDelivered::Unavailable)?,
            )
            .await
            .map_err(|_| NotDelivered::Unavailable)?
        {
            return Err(NotDelivered::Unavailable);
        }
        if let Err(error) = self.sink.send(operation, &p.wire).await {
            let _ = self.store.failed(&p, error).await;
            return Err(error);
        }
        self.store
            .ack(&p)
            .await
            .map_err(|_| NotDelivered::Unavailable)?;
        Ok(true)
    }
    pub async fn watermark_step(&mut self) -> std::result::Result<bool, NotDelivered> {
        if !self.enabled {
            return Err(NotDelivered::Unavailable);
        }
        let Some(delivery) = self
            .store
            .watermark(self.family)
            .await
            .map_err(|_| NotDelivered::Unavailable)?
        else {
            return Ok(false);
        };
        let w = &delivery.watermark;
        let now = u64::try_from(self.clock.unix_now()).map_err(|_| NotDelivered::Unavailable)?;
        if w.source_authority != self.source_authority {
            return Err(NotDelivered::Unauthenticated);
        }
        if !w.fresh(now) {
            return Err(NotDelivered::Unavailable);
        }
        self.admit(decimal(&w.projection_epoch, false).map_err(|_| NotDelivered::Malformed)?)?;
        let wire = w.encode(now).map_err(|_| NotDelivered::Malformed)?;
        if !self
            .store
            .current(
                self.family,
                decimal(&w.projection_epoch, false).map_err(|_| NotDelivered::Malformed)?,
                &delivery.token,
            )
            .await
            .map_err(|_| NotDelivered::Unavailable)?
        {
            return Err(NotDelivered::Unavailable);
        }
        let result = self
            .sink
            .send(Operation::PublishLeagueWatermarkV1, &wire)
            .await;
        let _ = self.store.release_watermark(&delivery).await;
        result?;
        Ok(true)
    }
    pub async fn run(mut self) {
        if !self.enabled {
            return;
        }
        let mut next_watermark = Duration::ZERO;
        let mut next_delivery = Duration::ZERO;
        let mut next_production = Duration::ZERO;
        let mut backoff = 1u64;
        loop {
            let now = self.clock.elapsed();
            if now >= next_watermark {
                let _ = self.watermark_step().await;
                next_watermark = self.clock.elapsed() + Duration::from_secs(1);
            }
            if self.family == "world_event_outcome" && self.clock.elapsed() >= next_production {
                let _ = self.store.produce().await;
                next_production = self.clock.elapsed() + Duration::from_secs(1);
            }
            if self.clock.elapsed() >= next_delivery {
                let start = self.clock.elapsed();
                let result = self.step().await;
                let delay = match &result {
                    Ok(_) => {
                        backoff = 1;
                        0
                    }
                    Err(NotDelivered::RateLimited | NotDelivered::Unavailable) => {
                        let delay = backoff;
                        backoff = (backoff * 2).min(10);
                        delay
                    }
                    Err(_) => 10,
                };
                if let Err(error) = result {
                    // Closed diagnostic vocabulary; never log a record, native evidence or id.
                    let elapsed = self.clock.elapsed().saturating_sub(start);
                    oteryn_error_codes::emit(
                        "oteryn-game-server",
                        &oteryn_error_codes::Line::new(
                            oteryn_error_codes::Level::Warn,
                            "admission",
                            "world_event_outcome_publish",
                        )
                        .code(error.code())
                        .field("family", oteryn_error_codes::FieldValue::Token(self.family))
                        .field(
                            "ms",
                            oteryn_error_codes::FieldValue::U64(
                                u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
                            ),
                        ),
                    );
                }
                next_delivery = self.clock.elapsed() + Duration::from_secs(delay);
            }
            self.clock.sleep(Duration::from_millis(100)).await;
        }
    }
}
