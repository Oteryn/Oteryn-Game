//! A planned reset signal carries only the durable owner's sealed RETIRING
//! witness. Ordinary process shutdown cannot request it.
use crate::durability::world_reset::RetiringWorldReset;
use oteryn_foundation::CancellationToken;
use std::sync::Mutex;

#[derive(Default)]
pub struct GameplayDrain {
    reset: Mutex<Option<RetiringWorldReset>>,
    epoch: u64,
    requested: CancellationToken,
}

impl GameplayDrain {
    /// Boot supplies the durable active epoch before any map view is served.
    pub fn at_reset_epoch(epoch: u64) -> Self {
        Self {
            epoch,
            ..Self::default()
        }
    }

    pub(super) fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn request(&self, reset: RetiringWorldReset) {
        let mut current = self
            .reset
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if current.is_none() {
            *current = Some(reset);
            self.requested.cancel();
        }
    }

    pub(super) fn reset(&self) -> Option<RetiringWorldReset> {
        *self
            .reset
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    pub(super) async fn requested(&self) {
        self.requested.cancelled().await;
    }
}
