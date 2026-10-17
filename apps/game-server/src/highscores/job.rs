//! One World's HIGHSCORES0 job. The composition root owns its dedicated projection-role Root
//! and the actual planned-reset notifications. No client command can start or force this job.

use super::{DurabilityRoot, SnapshotCatalogues, SnapshotError, SnapshotInfo};
use crate::foundation::WorldId;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{Instant, timeout_at};

pub(crate) struct HighscoreWorldJob {
    root: Arc<DurabilityRoot>,
    world: WorldId,
    catalogues: SnapshotCatalogues,
    #[cfg(test)]
    completed_passes: std::sync::atomic::AtomicU64,
}

impl HighscoreWorldJob {
    pub(crate) fn new(
        root: Arc<DurabilityRoot>,
        world: WorldId,
        catalogues: SnapshotCatalogues,
    ) -> Self {
        Self {
            root,
            world,
            catalogues,
            #[cfg(test)]
            completed_passes: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Check durable due state after restart, using database time and the publication locks.
    pub(crate) async fn periodic_tick(&self) -> Result<Vec<SnapshotInfo>, SnapshotError> {
        self.root
            .refresh_highscore_world(self.world, self.catalogues.clone(), false)
            .await
    }

    /// Called only from the composition root's actual planned World reset notification.
    pub(crate) async fn after_planned_reset(&self) -> Result<Vec<SnapshotInfo>, SnapshotError> {
        self.root
            .refresh_highscore_world(self.world, self.catalogues.clone(), true)
            .await
    }

    #[cfg(test)]
    pub(crate) fn completed_passes(&self) -> u64 {
        self.completed_passes
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// First check is immediate. Every successful check schedules from durable database time,
    /// including a recent snapshot skipped after restart. A failed periodic pass backs off one
    /// interval; a failed reset preserves the pending periodic deadline. Closing resets stops.
    pub(crate) async fn run(&self, mut planned_resets: mpsc::Receiver<()>) {
        let interval = Duration::from_secs(30 * 60);
        let mut due = Instant::now();
        loop {
            let reset = match timeout_at(due, planned_resets.recv()).await {
                Ok(Some(())) => true,
                Ok(None) => break,
                Err(_) => false,
            };
            // Anchor before the database pass. Subtracting its elapsed time is conservative:
            // connection/lock delay can cause an early recheck, never postpone durable expiry.
            let started = Instant::now();
            let result = self
                .root
                .refresh_highscore_schedule(self.world, self.catalogues.clone(), reset)
                .await;
            match result {
                Ok(refresh) => due = started + refresh.next_check_after,
                Err(error) => {
                    if !reset {
                        due = Instant::now() + interval;
                    }
                    eprintln!(
                        "highscore snapshot pass failed world={:?}: {error:?}",
                        self.world
                    );
                }
            }
            #[cfg(test)]
            self.completed_passes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
}
