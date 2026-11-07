//! WORLD-INTERACTION-0 §6.3: rope-spot ascent uses the existing movement owner.
use super::*;

// canary:data/global.lua:83-84 and data/libs/functions/tile.lua:96.
const ROPE_GROUND: &[u32] = &[
    386, 421, 7762, 12202, 12936, 14238, 17238, 21965, 21966, 21967, 21968, 23363,
];
// Accepted §6.3 order: south, north, east, west, then the diagonals.
const OFFSETS: &[(i32, i32)] = &[
    (0, 1),
    (0, -1),
    (1, 0),
    (-1, 0),
    (-1, 1),
    (1, 1),
    (-1, -1),
    (1, -1),
];

struct RopeProof<'a> {
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    spot: LogicalCell,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    destination: MovementLocalPosition,
}

/// Qualify a resolved, in-reach rope spot and a free upstairs landing. This produces the same
/// sealed position successor as floor traversal; no respawn or spell state is cleared.
#[allow(
    clippy::too_many_arguments,
    reason = "independently resolved owner facts"
)]
pub fn prepare_bundle_rope<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    spot: LogicalCell,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let mut proof = RopeProof {
        cells,
        actor,
        session,
        expected,
        spot,
        pz_blocked,
        closed,
        destination: expected.position(),
    };
    proof.destination = proof.resolve(runtime)?;
    Ok(proof)
}

impl RopeProof<'_> {
    fn resolve(&self, runtime: &ChannelRuntimeV1) -> Result<MovementLocalPosition, MovementError> {
        qualify_origin(runtime, self.cells, self.actor, self.session, self.expected)?;
        let origin = self.expected.position();
        if i32::from(origin.floor) != self.spot.z
            || origin.x.abs_diff(self.spot.x) > 1
            || origin.y.abs_diff(self.spot.y) > 1
        {
            return Err(MovementError::NotQualified);
        }
        let tile = self
            .cells
            .spell_tiles()
            .lookup(self.cells.scope(), self.spot)
            .map_err(|_| MovementError::NotQualified)?;
        if !tile.ground_present()
            || !(tile
                .ground_source_id()
                .is_some_and(|id| ROPE_GROUND.contains(&id))
                || tile.top_source_ids().contains(&12935))
        {
            return Err(MovementError::NotQualified);
        }
        let floor = self
            .spot
            .z
            .checked_sub(1)
            .filter(|z| (0..=MAX_Z).contains(z))
            .ok_or(MovementError::CoordinateOverflow)?;
        let ground = ground_of(self.cells);
        let landable = landable_of(self.cells, self.pz_blocked, self.closed);
        for &(dx, dy) in OFFSETS {
            let (Some(x), Some(y)) = (self.spot.x.checked_add(dx), self.spot.y.checked_add(dy))
            else {
                continue;
            };
            let next = MovementLocalPosition {
                x,
                y,
                floor: i16::try_from(floor).map_err(|_| MovementError::CoordinateOverflow)?,
            };
            if ground((x, y, floor)) == Ground::Walkable
                && landable((x, y, floor))
                && !runtime
                    .position_occupied_by_other(self.actor, next)
                    .map_err(MovementError::Actor)?
            {
                return Ok(next);
            }
        }
        Err(MovementError::Blocked)
    }
}

impl crate::foundation::source_step_seal::Sealed for RopeProof<'_> {}
impl crate::foundation::SourceStepCommitProof for RopeProof<'_> {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError> {
        if self.resolve(runtime).ok() != Some(self.destination) {
            return Err(CarrierError::PlanConflict);
        }
        Ok(())
    }

    fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
        MovementFacing,
    ) {
        (
            self.actor,
            self.expected,
            self.destination,
            self.expected.facing().unwrap_or(MovementFacing::South),
        )
    }
}
