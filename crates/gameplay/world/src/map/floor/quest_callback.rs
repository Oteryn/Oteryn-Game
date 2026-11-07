//! Callback relocation uses the same sealed movement commit as stairs and rope.
use super::*;

struct CallbackTeleport<'a> {
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    facing: Option<MovementFacing>,
    opening_door: Option<u64>,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_quest_callback_teleport<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    prepare_quest_callback_teleport_facing(
        runtime,
        cells,
        actor,
        session,
        expected,
        destination,
        pz_blocked,
        closed,
        None,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "the proof binds the current actor, movement scope, landing rules and facing"
)]
pub fn prepare_quest_callback_teleport_facing<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    facing: Option<MovementFacing>,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let proof = CallbackTeleport {
        cells,
        actor,
        session,
        expected,
        destination,
        pz_blocked,
        closed,
        facing,
        opening_door: None,
    };
    proof.check(runtime)?;
    Ok(proof)
}

/// Prepare against the World owner's hypothetical opening of one exact doorway.
/// Static blockers, other doors, scope, PZ rules and runtime occupancy still apply.
#[allow(clippy::too_many_arguments)]
pub fn prepare_quest_door_opening<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    opening_door: u64,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let proof = CallbackTeleport {
        cells,
        actor,
        session,
        expected,
        destination,
        pz_blocked,
        closed,
        facing: None,
        opening_door: Some(opening_door),
    };
    proof.check(runtime)?;
    Ok(proof)
}

impl CallbackTeleport<'_> {
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        qualify_origin(runtime, self.cells, self.actor, self.session, self.expected)?;
        let to = self.destination;
        let cell = (to.x, to.y, i32::from(to.floor));
        let ground = if let Some(key) = self.opening_door {
            let NativeMovementCollisionIndex::Bundle(index) = self.cells.index() else {
                return Err(MovementError::NotQualified);
            };
            let (door, _) =
                super::super::view::placement_of(key).ok_or(MovementError::NotQualified)?;
            if pos(to.x, to.y, i32::from(to.floor)) != Some(door)
                || index.map().facts.doors().is_open(key) != Some(false)
            {
                return Err(MovementError::NotQualified);
            }
            index
                .lookup_except_door(
                    self.cells.scope(),
                    LogicalCell {
                        x: to.x,
                        y: to.y,
                        z: i32::from(to.floor),
                    },
                    key,
                )
                .map_err(MovementError::Cell)?
                == CollisionClass::Walkable
        } else {
            ground_of(self.cells)(cell) == Ground::Walkable
        };
        if !ground
            || !landable_of(self.cells, self.pz_blocked, self.closed)(cell)
            || runtime
                .position_occupied_by_other(self.actor, to)
                .map_err(MovementError::Actor)?
        {
            return Err(MovementError::Blocked);
        }
        Ok(())
    }
}

impl crate::foundation::source_step_seal::Sealed for CallbackTeleport<'_> {}
impl crate::foundation::SourceStepCommitProof for CallbackTeleport<'_> {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError> {
        self.check(runtime).map_err(|_| CarrierError::PlanConflict)
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
            self.facing
                .or(self.expected.facing())
                .unwrap_or(MovementFacing::South),
        )
    }
}
