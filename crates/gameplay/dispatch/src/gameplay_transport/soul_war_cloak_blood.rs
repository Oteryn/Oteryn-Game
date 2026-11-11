//! Consume actual committed Cloak HP callbacks before the next native AI turn.
use super::ComposedFreshAdmission;
use crate::durability::creature_source_items::CloakBloodCallback;
pub(super) mod step;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn drain_cloak_blood_events(&self) -> bool {
        let events = match self.runtime.lock().await.pending_cloak_blood_events(32) {
            Ok(events) => events,
            Err(_) => return false,
        };
        if events.is_empty() {
            return true;
        }
        // Bootstrap through the existing owner before taking its physical lane.
        if !self.ensure_source_map_initialized().await {
            return false;
        }
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        self.drain_cloak_blood_with_permit(&permit).await
    }

    pub(super) async fn drain_cloak_blood_with_permit(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> bool {
        let events = match self.runtime.lock().await.pending_cloak_blood_events(32) {
            Ok(events) => events,
            Err(_) => return false,
        };
        for event in events {
            let callback = {
                let runtime = self.runtime.lock().await;
                let Some(room) = self.effective_rooms().gate_monster_ai() else {
                    return false;
                };
                let Some(native) = self
                    .active_generation
                    .and_then(|active| active.native_gameplay())
                else {
                    return false;
                };
                match CloakBloodCallback::from_committed_health(&runtime, room, native, &event) {
                    Ok(callback) => callback,
                    Err(error) => {
                        #[cfg(test)]
                        eprintln!("Cloak physical callback qualification: {error:?}");
                        #[cfg(not(test))]
                        let _ = error;
                        return false;
                    }
                }
            };
            let created = match self
                .root
                .commit_cloak_blood(permit, self.character, self.holder, &callback)
                .await
            {
                Ok((_, created)) => created,
                Err(error) => {
                    #[cfg(test)]
                    eprintln!("Cloak physical callback commit: {error:?}");
                    #[cfg(not(test))]
                    let _ = error;
                    return false;
                }
            };
            self.states.loop_ground.invalidate();
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if !callback.current_for(&runtime)
                || runtime.acknowledge_cloak_blood_event(&event).is_err()
            {
                return false;
            }
            if created {
                let p = event.position();
                states.cloak_blood_cells.insert((p.x, p.y, p.floor));
            }
        }
        let empty = self
            .runtime
            .lock()
            .await
            .pending_cloak_blood_events(1)
            .is_ok_and(|events| events.is_empty());
        if !empty {
            self.states
                .spell_states
                .lock()
                .await
                .owner_wake
                .notify_one();
        }
        empty
    }
}
