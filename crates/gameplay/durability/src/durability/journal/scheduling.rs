//! One bounded batch, distributed across at most four serial consumers.
use super::*;
use oteryn_gameplay_native_admission_source::{
    SourceError,
    journal::{
        ReadGameJournalCursorV1Response,
        publisher::{JournalOutbox, JournalPeer, Publisher, Step, StoredPublication},
    },
};
use std::{
    future::{Future, poll_fn},
    task::Poll,
    time::{Duration, Instant},
};
const BATCH: u8 = 64;
const BUDGET: Duration = Duration::from_secs(24);
async fn retry_source<T>(
    mut run: impl AsyncFnMut() -> std::result::Result<T, SourceError>,
) -> std::result::Result<T, SourceError> {
    for _ in 0..40 {
        match run().await {
            Err(SourceError::Unavailable) => tokio::time::sleep(Duration::from_millis(5)).await,
            other => return other,
        }
    }
    Err(SourceError::Unavailable)
}
struct Claimed<'a, 'r, 'f, 's> {
    store: &'a ProducerStore<'r, 'f, 's>,
    claim: &'a DeliveryClaim,
}
impl JournalOutbox for Claimed<'_, '_, '_, '_> {
    async fn current(&self, s: &ReadGameJournalCursorV1) -> std::result::Result<bool, SourceError> {
        retry_source(async || {
            if !self
                .store
                .claim_current(self.claim)
                .await
                .map_err(|_| SourceError::Unavailable)?
            {
                return Ok(false);
            }
            self.store.current(s).await
        })
        .await
    }
    async fn first_for_replay(
        &self,
        s: &ReadGameJournalCursorV1,
    ) -> std::result::Result<Option<StoredPublication>, SourceError> {
        retry_source(async || self.store.first_for_replay(s).await).await
    }
    async fn after_cursor(
        &self,
        s: &ReadGameJournalCursorV1,
        c: &ReadGameJournalCursorV1Response,
    ) -> std::result::Result<Option<StoredPublication>, SourceError> {
        retry_source(async || self.store.after_cursor(s, c).await).await
    }
    async fn acknowledge(
        &self,
        s: &ReadGameJournalCursorV1,
        r: &PublishGameJournalRecordV1,
    ) -> std::result::Result<(), SourceError> {
        retry_source(async || self.store.acknowledge(s, r).await).await
    }
}
impl ProducerStore<'_, '_, '_> {
    pub async fn deliver_due<P: JournalPeer>(
        &self,
        source: &str,
        publishers: &mut [Publisher<P>],
    ) -> Result<usize> {
        if publishers.is_empty() || publishers.len() > 4 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let claims = self.claim_due(source, BATCH).await?;
        let mut lanes = (0..publishers.len())
            .map(|_| Vec::new())
            .collect::<Vec<_>>();
        let count = lanes.len();
        for (index, claim) in claims.into_iter().enumerate() {
            lanes[index % count].push(claim);
        }
        let until = Instant::now() + BUDGET;
        let mut futures = publishers
            .iter_mut()
            .zip(lanes)
            .map(|(publisher, lane)| {
                Some(Box::pin(DurabilityRoot::with_holder_owner(
                    self.deliver_lane(publisher, lane, until),
                )))
            })
            .collect::<Vec<_>>();
        let mut total = 0;
        let mut error = None;
        poll_fn(|cx| {
            for slot in &mut futures {
                if let Some(future) = slot
                    && let Poll::Ready(result) = future.as_mut().poll(cx)
                {
                    *slot = None;
                    match result {
                        Ok(count) => total += count,
                        Err(e) => error = Some(e),
                    }
                }
            }
            if futures.iter().all(Option::is_none) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
        if let Some(error) = error {
            return Err(error);
        }
        Ok(total)
    }
    async fn deliver_lane<P: JournalPeer>(
        &self,
        publisher: &mut Publisher<P>,
        claims: Vec<DeliveryClaim>,
        until: Instant,
    ) -> Result<usize> {
        let mut done = 0;
        for claim in claims {
            if Instant::now() >= until {
                self.release_retry(&claim, until + Duration::from_secs(3))
                    .await?;
                continue;
            }
            publisher.restore_retry_attempt(claim.attempt);
            let start = Instant::now();
            let step = match tokio::time::timeout_at(
                tokio::time::Instant::from_std(until),
                publisher.step(
                    &claim.stream,
                    &Claimed {
                        store: self,
                        claim: &claim,
                    },
                ),
            )
            .await
            {
                Ok(step) => step,
                Err(_) => {
                    self.release_retry(&claim, until + Duration::from_secs(3))
                        .await?;
                    continue;
                }
            };
            let (delay, progress, quarantine) = match step {
                Step::Accepted => (Duration::ZERO, true, false),
                Step::Idle => (
                    Duration::from_secs(28).saturating_sub(start.elapsed()),
                    true,
                    false,
                ),
                Step::RetryAfter(delay) => (delay, false, false),
                Step::Disabled => (Duration::from_secs(28), false, false),
                Step::Quarantined => {
                    eprintln!("event=game_journal_quarantined reason=authority_or_protocol");
                    (Duration::ZERO, false, true)
                }
            };
            // Do not release ownership until the disposition is durable. A stale
            // token can never alter a successor's deadline or quarantine state.
            loop {
                match self
                    .finish(
                        &claim,
                        publisher.retry_attempt(),
                        delay,
                        progress,
                        quarantine,
                    )
                    .await
                {
                    Ok(_) => break,
                    Err(DurabilityError::RootUnavailable)
                        if Instant::now() < until + Duration::from_secs(3) =>
                    {
                        tokio::time::sleep(Duration::from_millis(5)).await
                    }
                    Err(error) => return Err(error),
                }
            }
            done += 1;
        }
        Ok(done)
    }
    async fn release_retry(&self, claim: &DeliveryClaim, until: Instant) -> Result<()> {
        loop {
            match self.release_claim(claim).await {
                Err(DurabilityError::RootUnavailable) if Instant::now() < until => {
                    tokio::time::sleep(Duration::from_millis(5)).await
                }
                other => return other,
            }
        }
    }
}
