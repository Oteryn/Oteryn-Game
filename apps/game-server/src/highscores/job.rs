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

    /// First check is immediate. Dropping the composition root's reset sender stops the task.
    /// A failed pass leaves the prior durable generation and the next scheduled pass can try
    /// again. Missed intervals are skipped; there is no immediate retry loop.
    pub(crate) async fn run(&self, mut planned_resets: mpsc::Receiver<()>) {
        let interval = Duration::from_secs(30 * 60);
        let mut due = Instant::now();
        loop {
            let result = match timeout_at(due, planned_resets.recv()).await {
                Ok(Some(())) => {
                    let result = self.after_planned_reset().await;
                    if result.is_ok() {
                        due = Instant::now() + interval;
                    }
                    result
                }
                Ok(None) => break,
                Err(_) => {
                    let result = self.periodic_tick().await;
                    due = Instant::now() + interval;
                    result
                }
            };
            if let Err(error) = result {
                eprintln!(
                    "highscore snapshot pass failed world={:?}: {error:?}",
                    self.world
                );
            }
        }
    }
}
