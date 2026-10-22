//! A planned reset signal carries only the durable owner's sealed RETIRING
//! witness. Ordinary process shutdown cannot request it.
use crate::durability::world_reset::RetiringWorldReset;
use oteryn_foundation::CancellationToken;
use std::sync::Mutex;

#[derive(Default)]
pub struct GameplayDrain {
    reset: Mutex<Option<RetiringWorldReset>>,
    requested: CancellationToken,
}

impl GameplayDrain {
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
        self.reset
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    pub(super) async fn requested(&self) {
        self.requested.cancelled().await;
    }
}
