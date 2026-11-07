//! Soulpit uses the existing world owner, independently of connected player loops.
use super::super::{ComposedFreshAdmission, world_respawns::stopping_first};
use crate::monster_owner_cycle::respawn_job::RespawnExit;
use oteryn_foundation::CancellationToken;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn run_soulpits(
        &self,
        shutdown: &CancellationToken,
    ) -> RespawnExit {
        // Crystal's global encounter tick is independent of addEvent's exact deadlines.
        let mut global_due = self.owner_now().get().saturating_add(1_000_000);
        loop {
            let now = self.owner_now().get();
            let global_tick = now >= global_due;
            if global_tick {
                global_due = now.saturating_add(1_000_000);
            }
            let deadline = if let Some(owner) = self.states.boss_activity_handoff.get() {
                match stopping_first(owner.drive_soulpit(self, now, global_tick), shutdown).await {
                    None => return RespawnExit::Shutdown,
                    Some(Ok(deadline)) if deadline.is_none_or(|due| due > now) => deadline,
                    Some(Ok(_)) | Some(Err(())) => {
                        self.runtime.lock().await.retire_owner_cycle();
                        return RespawnExit::StaleOwner;
                    }
                }
            } else {
                None
            };
            // Crystal 201189f data/scripts/globalevents/encounters.lua:12. The activity
            // owner also wakes exactly for the separate four-second spawn and ten-minute
            // kick events; only global ticks may clear a wave after its 4.5-second gate.
            let due = deadline.map_or(global_due, |deadline| deadline.min(global_due));
            if stopping_first(
                tokio::time::sleep(std::time::Duration::from_micros(
                    due.saturating_sub(self.owner_now().get()),
                )),
                shutdown,
            )
            .await
            .is_none()
            {
                return RespawnExit::Shutdown;
            }
        }
    }
}
