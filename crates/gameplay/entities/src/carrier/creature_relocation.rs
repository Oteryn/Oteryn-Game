//! Creature-only compare commit and read-only spawn origin; gameplay policy stays in World.
use super::*;

impl CurrentOwnerMovementPosition<'_> {
    pub fn commit_creature_relocation(
        &mut self,
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
        next: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.relocate_creature(actor, expected, next, None)
    }

    /// Publish a source teleport's position and pose in the same movement occurrence.
    pub fn commit_creature_relocation_facing(
        &mut self,
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
        next: MovementLocalPosition,
        facing: MovementFacing,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.relocate_creature(actor, expected, next, Some(facing))
    }

    fn relocate_creature(
        &mut self,
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
        next: MovementLocalPosition,
        facing: Option<MovementFacing>,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        if !self.carrier.creature_alive(self.continuity, actor) {
            return Err(CarrierError::NotCreature);
        }
        if actor.0 != expected.0.actor_ref {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        let next = LocalPosition {
            x: next.x,
            y: next.y,
            floor: next.floor,
        };
        let result = if let Some(facing) = facing {
            self.carrier.compare_commit_position_with_facing(
                self.continuity,
                expected.0,
                next,
                facing,
            )
        } else {
            self.carrier.compare_commit_position(
                self.continuity,
                expected.0,
                expected.0.version.context,
                next,
            )
        };
        result.map(MovementPositionSnapshot)
    }
}

impl ChannelRuntimeV1 {
    /// Original native spawn origin on the retained lethal turn. The exact dead
    /// generation and pinned context must still exist; encounter copies have no
    /// static spawn origin and cannot impersonate a native quest target.
    pub fn creature_death_spawn_cell(&self, actor: ExactActorRef) -> Option<MovementLocalPosition> {
        if !self.retains_corpse_projection(actor)
            || self.companion_snapshot_including_dead(actor).ok()?.health != 0
        {
            return None;
        }
        if let Some(spawn) = self
            .boss_spawns
            .iter()
            .find(|spawn| spawn.actor == Some(actor))
        {
            return Some(spawn.position);
        }
        let (source, index) = self.carrier.locate_spawn_cell(actor)?;
        let cell = self
            .carrier
            .spawn_definition(source)
            .ok()?
            .placement_cells
            .get(index)?;
        Some(MovementLocalPosition {
            x: cell.x,
            y: cell.y,
            floor: cell.floor,
        })
    }

    /// Actual spawn placement, never the creature's admission or last observed cell.
    pub fn creature_spawn_cell(&self, actor: ExactActorRef) -> Option<MovementLocalPosition> {
        if !self.contains_live_creature(actor) {
            return None;
        }
        if let Some(spawn) = self
            .boss_spawns
            .iter()
            .find(|spawn| spawn.actor == Some(actor))
        {
            return Some(spawn.position);
        }
        let (source, index) = self.carrier.locate_spawn_cell(actor)?;
        let cell = self
            .carrier
            .spawn_definition(source)
            .ok()?
            .placement_cells
            .get(index)?;
        Some(MovementLocalPosition {
            x: cell.x,
            y: cell.y,
            floor: cell.floor,
        })
    }
}
