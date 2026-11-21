//! Eight-way neighbor commits retain the existing exact-snapshot carrier authority.
use super::*;
impl CurrentOwnerMovementPosition<'_> {
    /// Commit one same-floor neighboring step after the Movement owner's collision proof.
    /// The cardinal-only entry remains available to existing callers.
    pub fn commit_step(
        &mut self,
        expected: MovementPositionSnapshot,
        next: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let before = expected.position();
        if next.floor != before.floor
            || next.x.abs_diff(before.x) > 1
            || next.y.abs_diff(before.y) > 1
            || (next.x == before.x && next.y == before.y)
        {
            return Err(CarrierError::MovementNonCardinal);
        }
        self.carrier.commit_movement_position(
            self.continuity,
            expected.0,
            LocalPosition {
                x: next.x,
                y: next.y,
                floor: next.floor,
            },
        )
    }
}
