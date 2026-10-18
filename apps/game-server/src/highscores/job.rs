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

    #[cfg(test)]
    #[allow(
        dead_code,
        reason = "the PostgreSQL job cases in character_authority_postgres read it"
    )]
    pub(crate) fn completed_passes(&self) -> u64 {
        self.completed_passes
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// First check is immediate. Every successful check schedules from durable database time,
    /// including a recent snapshot skipped after restart. A failed periodic pass backs off one
    /// interval; a failed reset preserves the pending periodic deadline. A message on
    /// `planned_resets` is the composition root's actual planned World reset notification and
    /// forces one complete cut; closing the channel stops the job.
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
                Ok(refresh) => {
                    for snapshot in &refresh.snapshots {
                        published(self.world, snapshot);
                    }
                    due = started + refresh.next_check_after;
                }
                Err(error) => {
                    if !reset {
                        due = Instant::now() + interval;
                    }
                    let reason = match &error {
                        SnapshotError::Unavailable(source) => format!("unavailable {source:?}"),
                        other => format!("{other:?}"),
                    };
                    eprintln!(
                        "event=highscore_snapshot state=failed world={:?} reset={reset} reason=\"{reason}\"",
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

/// One operator event per published (World, category) snapshot.
fn published(world: WorldId, snapshot: &SnapshotInfo) {
    eprintln!(
        "event=highscore_snapshot state=published world={world:?} category={} snapshot_id={} computed_at_ms={} source_cut={}",
        snapshot.category, snapshot.snapshot_id, snapshot.computed_at_ms, snapshot.source_cut
    );
}
