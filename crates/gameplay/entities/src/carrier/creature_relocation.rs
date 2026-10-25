//! Creature-only compare commit and read-only spawn origin; gameplay policy stays in World.
use super::*;

impl CurrentOwnerMovementPosition<'_> {
    pub fn commit_creature_relocation(
        &mut self,
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
        next: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        if !self.carrier.creature_alive(self.continuity, actor) {
            return Err(CarrierError::NotCreature);
        }
        if actor.0 != expected.0.actor_ref {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        self.carrier
            .compare_commit_position(
                self.continuity,
                expected.0,
                expected.0.version.context,
                LocalPosition {
                    x: next.x,
                    y: next.y,
                    floor: next.floor,
                },
            )
            .map(MovementPositionSnapshot)
    }
}

impl ChannelRuntimeV1 {
    /// Actual spawn placement, never the creature's admission or last observed cell.
    pub fn creature_spawn_cell(&self, actor: ExactActorRef) -> Option<MovementLocalPosition> {
        if !self.contains_live_creature(actor) {
            return None;
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
