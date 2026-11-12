//! PLAT-WIRE-1: immutable public aggregate samples over the public-projection transport.
use super::*;

pub const PATH: &str = "/internal/v1/game-public/world-totals";
pub const REQUEST_BYTES: usize = 1_024;
pub const BATCH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotalsSnapshot {
    pub world_id: String,
    pub projection_epoch: u64,
    pub projection_revision: u64,
    pub source_observed_at: i64,
    pub online_players: u64,
    pub player_deaths: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct TotalsFacts {
    pub projection_epoch: u64,
    pub complete_through: i64,
    pub observed_at: i64,
}

#[derive(Serialize)]
struct Wire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    world_id: &'a str,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    online_players: String,
    player_deaths: String,
}

pub fn encode_totals(authority: &str, s: &TotalsSnapshot) -> Result<String, SourceError> {
    if !envelope_valid(
        authority,
        s.projection_epoch,
        s.projection_revision,
        s.source_observed_at,
    ) || !canonical_uuid(&s.world_id, Some(b'7'))
        || [
            s.projection_epoch,
            s.projection_revision,
            s.online_players,
            s.player_deaths,
        ]
        .iter()
        .any(|v| *v > i64::MAX as u64)
    {
        return Err(SourceError::InvalidInput);
    }
    finish(
        serde_json::to_string(&Wire {
            contract_version: 1,
            operation: "PublishWorldTotalsV1",
            source_authority: authority,
            world_id: &s.world_id,
            projection_epoch: s.projection_epoch.to_string(),
            projection_revision: s.projection_revision.to_string(),
            source_observed_at: s.source_observed_at.to_string(),
            online_players: s.online_players.to_string(),
            player_deaths: s.player_deaths.to_string(),
        }),
        REQUEST_BYTES,
    )
}

impl<C: ReportClock, S: PublicStore, K: PublicSink, F: EpochFence> Publisher<C, S, K, F> {
    /// Delivery never deletes history: only the exact immutable sample is acknowledged.
    /// The lane gate covers transport failures without stopping durable sampling.
    pub async fn totals_step(&self) -> Result<Delivery, NotDelivered> {
        let rows = self
            .pass(|| self.store.next_totals())
            .await
            .map_err(|()| NotDelivered::Unavailable)?;
        for row in rows {
            self.admit(row.projection_epoch)
                .map_err(|_| NotDelivered::Unavailable)?;
            let body = encode_totals(&self.source_authority, &row)
                .map_err(|_| NotDelivered::InvalidReport)?;
            let result = self
                .deliver(Operation::PublishWorldTotalsV1, &body, EXCHANGE_BOUND)
                .await;
            log(
                Family::WorldTotals,
                "PublishWorldTotalsV1",
                class(&result),
                Duration::ZERO,
            );
            result?;
            self.pass(|| self.store.ack_totals(&row))
                .await
                .map_err(|()| NotDelivered::Unavailable)?;
        }
        Ok(Delivery::Accepted)
    }

    pub(super) async fn totals_watermark(&self) -> Result<Delivery, NotDelivered> {
        let facts = self
            .pass(|| self.store.totals_facts())
            .await
            .map_err(|()| NotDelivered::Unavailable)?
            .ok_or(NotDelivered::Unavailable)?;
        self.admit(facts.projection_epoch)
            .map_err(|_| NotDelivered::Unavailable)?;
        let body = encode_watermark(
            &self.source_authority,
            Family::WorldTotals,
            facts.projection_epoch,
            facts.complete_through,
            facts.observed_at,
        )
        .map_err(|_| NotDelivered::InvalidReport)?;
        self.deliver(
            Operation::PublishPublicProjectionWatermarkV1,
            &body,
            EXCHANGE_BOUND,
        )
        .await
    }

    pub(super) async fn totals_loop(&self) {
        join2(self.totals_sample_loop(), self.totals_delivery_loop()).await;
    }

    // Sampling progresses independently of HTTP retries and historical replay.
    async fn totals_sample_loop(&self) {
        let mut cursor = String::new();
        let mut sample_due = Duration::ZERO;
        loop {
            let now = self.clock.elapsed();
            if now >= sample_due {
                match self.pass(|| self.store.sample_totals(&cursor)).await {
                    Ok(Some(last)) => cursor = last,
                    Ok(None) => {
                        cursor.clear();
                        sample_due = self.clock.elapsed() + ONLINE_INTERVAL;
                    }
                    Err(()) => {}
                }
            }
            self.clock.sleep(IDLE).await;
        }
    }

    async fn totals_delivery_loop(&self) {
        let mut watermark_due = Duration::ZERO;
        let mut gate = Gate::default();
        loop {
            if self.clock.elapsed() >= gate.at {
                let result = self.totals_step().await;
                self.watermark_settled(&mut gate, Family::WorldTotals, &result);
                if result.is_ok() && self.clock.elapsed() >= watermark_due {
                    let result = self.totals_watermark().await;
                    self.watermark_settled(&mut gate, Family::WorldTotals, &result);
                    watermark_due = self.clock.elapsed() + WATERMARK_PERIOD;
                }
            }
            self.clock.sleep(IDLE).await;
        }
    }
}
