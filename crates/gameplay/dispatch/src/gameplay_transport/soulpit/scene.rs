//! Read the current shared Map/Item owners before borrowing the physical kernel.
use super::super::ComposedFreshAdmission;
#[cfg(test)]
use super::runtime::ActivityHandoffScene;
use crate::foundation::GameSessionId;
#[cfg(test)]
use crate::foundation::{ExactActorRef, MovementLocalPosition};
#[cfg(test)]
use std::collections::BTreeMap;

#[cfg(test)]
pub(super) struct NativeScene {
    room: Option<std::sync::Arc<crate::content::QualifiedNativeEntryRoom>>,
    tiles: BTreeMap<(i32, i32, i16), (bool, bool)>,
    targets: Vec<(ExactActorRef, ExactActorRef)>,
    attributes: Vec<((ExactActorRef, GameSessionId), i32)>,
}
#[cfg(test)]
impl ActivityHandoffScene for NativeScene {
    fn emit_spawn_effect(&self, at: MovementLocalPosition, effect: u16) -> bool {
        self.room
            .as_ref()
            .is_some_and(|room| super::super::soul_war_scene::emit_effect(room, at, effect))
    }
    fn permits_spawn(&self, at: MovementLocalPosition) -> bool {
        self.tiles
            .get(&(at.x, at.y, at.floor))
            .is_some_and(|(walk, projectile)| *walk && !projectile)
    }
    fn attack_target(&self, actor: ExactActorRef) -> Option<ExactActorRef> {
        self.targets
            .iter()
            .find(|(source, _)| *source == actor)
            .map(|(_, target)| *target)
    }
    fn permits_condition(
        &self,
        source: ExactActorRef,
        target: ExactActorRef,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
    ) -> bool {
        self.targets.iter().any(|(actor, _)| *actor == source)
            && self
                .attributes
                .iter()
                .any(|((actor, _), _)| *actor == target)
            && crate::durability::native_world_read::source_sight_steps(from, to).is_ok_and(
                |steps| {
                    steps.iter().all(|(at, exempt)| {
                        *exempt
                            || self
                                .tiles
                                .get(&(at.x, at.y, at.floor))
                                .is_some_and(|(_, projectile)| !projectile)
                    })
                },
            )
    }
    fn player_attribute_damage_delta(
        &self,
        target: ExactActorRef,
        session: GameSessionId,
        _: u64,
    ) -> Option<i32> {
        self.attributes
            .iter()
            .find(|((actor, id), _)| *actor == target && *id == session)
            .map(|(_, delta)| *delta)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// One causal entry cue per genuine installed participant. The caller retains
    /// this pre-emission cursor across retries and carries it only over its barrier.
    pub(in crate::gameplay_transport) async fn emit_soulpit_entry_effects(
        &self,
        connections: &[(
            super::super::connection::AdmittedSession,
            super::super::connection::AdmittedSession,
        )],
    ) -> Result<Vec<(GameSessionId, u64)>, ()> {
        use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
        use crate::map::boot::BundleCollisionIndexExt as _;
        let room = self.rooms().gate_wire_position().ok_or(())?;
        if room.pinned_activity_source_room()
            != Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Soulpit)
        {
            return Err(());
        }
        let runtime = self.runtime.lock().await;
        runtime.owner_fence().map_err(|_| ())?;
        let active = self.active_generation.ok_or(())?;
        if !super::super::native_cue_offer::source_available(active, room, runtime.content_pin()) {
            return Err(());
        }
        let NativeMovementCollisionIndex::Bundle(index) = room.movement_cells().index() else {
            return Err(());
        };
        let map = index.map();
        let cursor = map
            .facts
            .movement_effects_since(0)
            .last()
            .map_or(0, |row| row.0);
        let mut positions = Vec::with_capacity(connections.len());
        for (_, next) in connections {
            let actor = next.runtime_actor.ok_or(())?;
            if !runtime
                .player_control_facts(actor, next.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return Err(());
            }
            let at = runtime
                .read_actor_position(actor)
                .map_err(|_| ())?
                .position();
            if at.floor != 8 || at.y != 31151 || !(32373..=32377).contains(&at.x) {
                return Err(());
            }
            positions.push((
                next.game_session_id,
                crate::map::overlay::TilePos {
                    x: u16::try_from(at.x).map_err(|_| ())?,
                    y: u16::try_from(at.y).map_err(|_| ())?,
                    floor: -8,
                },
            ));
        }
        for (_, at) in &positions {
            if !map.facts.emit_movement_effect(*at, 11) {
                return Err(());
            }
        }
        Ok(positions
            .into_iter()
            .map(|(session, _)| (session, cursor))
            .collect())
    }

    /// Descendant of committed entry and completed physical installation. The
    /// independently owned Instance map receives the source scripted state.
    pub(in crate::gameplay_transport) async fn activate_soulpit_obelisk(&self) -> Result<(), ()> {
        self.set_soulpit_obelisk(true).await
    }

    async fn set_soulpit_obelisk(&self, active: bool) -> Result<(), ()> {
        use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
        use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
        let room = self.rooms().gate_wire_position().ok_or(())?;
        if room.pinned_activity_source_room() != Some(PinnedActivitySourceRoom::Soulpit) {
            return Err(());
        }
        let generation = self.active_generation.ok_or(())?;
        let runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || !matches!(
                runtime.binding().runtime_scope(),
                crate::foundation::RuntimeScopeRefV1::Instance { .. }
            )
            || !super::super::native_cue_offer::source_available(
                generation,
                room,
                runtime.content_pin(),
            )
        {
            return Err(());
        }
        let NativeMovementCollisionIndex::Bundle(index) = room.movement_cells().index() else {
            return Err(());
        };
        use crate::map::boot::BundleCollisionIndexExt as _;
        let map = index.map();
        let tile = crate::map::view::compose_tile(
            &map.overlay,
            &map.facts,
            crate::map::overlay::TilePos {
                x: 32375,
                y: 31157,
                floor: -8,
            },
        )
        .map_err(|_| ())?
        .ok_or(())?;
        let mut glyphs = tile
            .entries
            .iter()
            .filter(|entry| matches!(entry.facts.appearance_id, 47367 | 47379));
        let glyph = glyphs.next().ok_or(())?;
        if glyphs.next().is_some() {
            return Err(());
        }
        let crate::map::view::EntrySource::Base { placement_key, .. } = glyph.source else {
            return Err(());
        };
        let target = if active { 47379 } else { 47367 };
        if glyph.facts.appearance_id == target {
            return Ok(());
        }
        map.facts
            .transform_movement_item(placement_key, target)
            .then_some(())
            .ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srx3_soulpit_source_zone_rejects_projectile_blocking_ground() {
        // Crystal zones.lua:22 and tile.lua:46: randomPosition excludes a
        // projectile-blocked tile even when ordinary movement can enter it.
        let at = MovementLocalPosition {
            x: 32374,
            y: 31145,
            floor: 8,
        };
        let scene = NativeScene {
            room: None,
            tiles: BTreeMap::from([((at.x, at.y, at.floor), (true, true))]),
            targets: Vec::new(),
            attributes: Vec::new(),
        };
        assert!(!scene.permits_spawn(at));
    }
}
