use super::*;
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Before the next AI turn, settle actual masterless Horse arrivals on31159.
    /// A failed/unknown durable call retains the exact actor and blocks the AI turn.
    pub(in crate::gameplay_transport) async fn extract_arrived_station_horses(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> bool {
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
            let mut found = Vec::new();
            for (actor, position, session) in census {
                if session.is_some()
                    || !runtime.matches_live_creature_identity(actor, b"oteryn:creature.horse")
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
                let pos = crate::map::overlay::TilePos { x, y, floor };
                if crate::map::view::compose_tile(&map.overlay, &map.facts.pinned(), pos)
                    .ok()
                    .flatten()
                    .is_some_and(|tile| {
                        tile.entries
                            .iter()
                            .any(|entry| entry.facts.appearance_id == 31159)
                    })
                {
                    found.push((actor, snapshot.position, snapshot.position_revision));
                }
            }
            (runtime.binding(), found)
        };
        if permit.check_scope(binding.runtime_scope()).is_err() {
            return false;
        }
        for (actor, at, revision) in arrivals {
            let occurrence = Sha256::new()
                .chain_update(b"oteryn:horse-station-catch:v1")
                .chain_update(self.world_id.as_bytes())
                .chain_update(self.channel_id.as_bytes())
                .chain_update(binding.scope_generation().get().to_be_bytes())
                .chain_update(actor.placement_identity())
                .finalize()
                .into();
            if self
                .root
                .catch_horse_station(
                    self.character,
                    self.holder,
                    HorseStationChannel {
                        world: self.world_id,
                        channel: self.channel_id,
                        generation: binding.scope_generation().get(),
                    },
                    occurrence,
                )
                .await
                .is_err()
            {
                return false;
            }
            let mut runtime = self.runtime.lock().await;
            if runtime.binding() != binding || runtime.owner_fence().is_err() {
                return false;
            }
            if runtime.companion_snapshot(actor).is_ok_and(|now| {
                now.position == at
                    && now.position_revision == revision
                    && now.state.master.is_none()
            }) && runtime.matches_live_creature_identity(actor, b"oteryn:creature.horse")
            {
                if let (Ok(x), Ok(y), Ok(floor)) = (
                    u16::try_from(at.x),
                    u16::try_from(at.y),
                    i8::try_from(-at.floor),
                ) {
                    let _ = map
                        .facts
                        .emit_movement_effect(crate::map::overlay::TilePos { x, y, floor }, 3);
                }
                if runtime.remove_arena_opponent(actor).is_err() {
                    return false;
                }
            }
        }
        true
    }
}
