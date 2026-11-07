use super::*;
use sha2::{Digest, Sha256};
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Runs before the next AI step, so an arrived golem cannot walk away from the
    /// source machine or deal another hit while its World observation is committing.
    pub(in crate::gameplay_transport) async fn extract_arrived_cake_golems(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> bool {
        if self
            .states
            .cake_stage
            .load(std::sync::atomic::Ordering::Relaxed)
            != 1
        {
            return true;
        }
        let now = now_seconds();
        if !oteryn_gameplay_foundation::scheduled_events::seasonal_events_at(now)
            .iter()
            .any(|event| {
                event.event == oteryn_protocol_oteryn::world_event::SeasonalEventKind::BirthdayCake
                    && event.active
            })
        {
            return true;
        }
        let Some(map) = self.bundle_map() else {
            return true;
        };
        let (binding, arrivals) = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err() {
                return false;
            }
            let Ok(census) = runtime.positioned_actor_census() else {
                return false;
            };
            let mut arrivals = Vec::new();
            for (actor, position, session) in census {
                if session.is_some()
                    || !runtime.matches_live_creature_identity(actor, GOLEM.as_bytes())
                    || machine_city(position.position()).is_none()
                {
                    continue;
                }
                let Ok(snapshot) = runtime.companion_snapshot(actor) else {
                    continue;
                };
                if snapshot.state.master.is_some() {
                    continue;
                }
                let at = position.position();
                let (Ok(x), Ok(y), Ok(floor)) = (
                    u16::try_from(at.x),
                    u16::try_from(at.y),
                    i8::try_from(-at.floor),
                ) else {
                    continue;
                };
                let pos = TilePos { x, y, floor };
                if crate::map::view::compose_tile(&map.overlay, &map.facts.pinned(), pos)
                    .ok()
                    .flatten()
                    .is_some_and(|tile| {
                        tile.entries
                            .iter()
                            .any(|entry| entry.facts.appearance_id == 8886)
                    })
                {
                    arrivals.push((actor, snapshot.position, snapshot.position_revision));
                }
            }
            (runtime.binding(), arrivals)
        };
        for (actor, at, revision) in arrivals {
            let occurrence = Sha256::new()
                .chain_update(b"oteryn:cake-golem-extraction:v1")
                .chain_update(binding.world_id().as_bytes())
                .chain_update(self.channel_id.as_bytes())
                .chain_update(binding.scope_generation().get().to_be_bytes())
                .chain_update(actor.placement_identity())
                .finalize()
                .into();
            let counted = match self
                .root
                .extract_cake_golem(
                    self.character,
                    self.holder,
                    permit,
                    self.world_id,
                    self.channel_id,
                    binding.scope_generation(),
                    occurrence,
                    now,
                )
                .await
            {
                Ok(counted) => counted,
                // Unknown commit retains the physical actor. The next owner pass replays this
                // exact observation before admitting another AI step or physical removal.
                Err(_) => return false,
            };
            if counted {
                let mut runtime = self.runtime.lock().await;
                if runtime.binding() != binding || runtime.owner_fence().is_err() {
                    return false;
                }
                if runtime.companion_snapshot(actor).is_ok_and(|current| {
                    current.position == at
                        && current.position_revision == revision
                        && current.state.master.is_none()
                }) && runtime.matches_live_creature_identity(actor, GOLEM.as_bytes())
                {
                    self.states.chat.cake_golem_crunch(&runtime, actor);
                    if let Some(map) = self.bundle_map()
                        && let (Ok(x), Ok(y), Ok(floor)) = (
                            u16::try_from(at.x),
                            u16::try_from(at.y),
                            i8::try_from(-at.floor),
                        )
                    {
                        let _ = map.facts.emit_movement_effect(TilePos { x, y, floor }, 32);
                    }
                    // Existing administrative native removal gives no corpse, loot or XP.
                    if runtime.remove_arena_opponent(actor).is_err() {
                        return false;
                    }
                }
            }
        }
        true
    }
}
