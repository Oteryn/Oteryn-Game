//! One World's HIGHSCORES0 job. The composition root owns its dedicated projection-role Root
//! and the actual planned-reset notifications. No client command can start or force this job.

use super::{DurabilityError, DurabilityRoot, SnapshotCatalogues, SnapshotError, SnapshotInfo};
use crate::foundation::WorldId;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{Instant, timeout_at};

/// The delay before a pass that found the shared Root's single holder busy
/// (`RootUnavailable`, e.g. the audit expiry or gameplay holding it) runs again. Contention is
/// transient, so it never waits a full interval: a fresh World is not left without snapshots,
/// and its highscore queries rejected, for 30 minutes.
const CONTENDED_RETRY: Duration = Duration::from_secs(5);

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
    /// interval; a failed reset preserves the pending periodic deadline. A pass that found the
    /// holder busy retries after [`CONTENDED_RETRY`] instead, a reset still as a reset. A message on
    /// `planned_resets` is the composition root's actual planned World reset notification and
    /// forces one complete cut; closing the channel stops the job.
    pub(crate) async fn run(&self, mut planned_resets: mpsc::Receiver<()>) {
        let interval = Duration::from_secs(30 * 60);
        let mut due = Instant::now();
        let mut retry_reset = false;
        loop {
            let reset = match timeout_at(due, planned_resets.recv()).await {
                Ok(Some(())) => true,
                Ok(None) => break,
                Err(_) => retry_reset,
            };
            retry_reset = false;
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
                    let contended = contended(&error);
                    retry_reset = reset && contended;
                    due = next_due_after_failure(contended, reset, Instant::now(), due, interval);
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

/// Whether a failed pass only found the shared Root's single holder busy.
fn contended(error: &SnapshotError) -> bool {
    matches!(
        error,
        SnapshotError::Unavailable(DurabilityError::RootUnavailable)
    )
}

/// The next pass after one failed at `now`. Any failure of a periodic pass, or of a reset whose
/// periodic deadline has already passed, backs off one `interval`; a failed reset keeps the
/// pending periodic `due`. A contended pass runs again after at most [`CONTENDED_RETRY`].
fn next_due_after_failure(
    contended: bool,
    reset: bool,
    now: Instant,
    due: Instant,
    interval: Duration,
) -> Instant {
    let pending = if reset && due > now {
        due
    } else {
        now + interval
    };
    if contended {
        pending.min(now + CONTENDED_RETRY)
    } else {
        pending
    }
}

/// One operator event per published (World, category) snapshot.
fn published(world: WorldId, snapshot: &SnapshotInfo) {
    eprintln!(
        "event=highscore_snapshot state=published world={world:?} category={} snapshot_id={} computed_at_ms={} source_cut={}",
        snapshot.category, snapshot.snapshot_id, snapshot.computed_at_ms, snapshot.source_cut
    );
}

#[cfg(test)]
mod job_tests;
